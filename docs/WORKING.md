# WORKING.md — How It Works (End-to-End)

## 1. Mental Model in 30 Seconds

```
User opens terminal → tui-preview ./project
  → Ratatui draws two panes
  → User moves cursor
  → Router asks “what handler for this file?”
  → Handler decodes (off main thread) → cached → rendered via terminal graphics
  → User never leaves keyboard, never spawns external app
```

Lightweight TUI shell + pure Rust decoders + smart cache = fast.

## 2. Startup Sequence — `src/main.rs:1`

```rust
fn main() -> Result<()> {
    // 1. Parse CLI via clap: tui-preview [PATH] [--theme] [--clear-cache]
    // 2. Load Config from ~/.config/tui-preview/config.toml (or defaults)
    // 3. Init tracing to ~/.cache/tui-preview/debug.log
    // 4. Resolve start path: arg or "." → canonicalize
    // 5. Detect terminal caps: term::capabilities::detect()
    // 6. Enter raw mode: crossterm::terminal::enable_raw_mode()
    // 7. Build Router via preview::build_router(&config)
    // 8. Build Cache via cache::Cache::open(cache_dir)
    // 9. Create App { files: fs::list_dir(path), selected: 0, ... }
    // 10. Tokio runtime loop: event_stream + render
}
```

Time budget: `<80ms` cold start on Windows, dominated by `list_dir` + `detect caps`.

## 3. Event Loop — `src/app.rs:50`

```
Loop at 60Hz (16ms tick) but render-on-demand:

tokio::select! {
  Some(Ok(Event::Key(k))) = event_stream.next() => app.handle_key(k),
  Some(preview_result) = preview_rx.recv()      => app.apply_preview(preview_result),
  _ = interval.tick()                           => if app.dirty { render() },
  _ = cancel_token.cancelled()                  => abort job
}
```

`handle_key` never does I/O; it enqueues `Action` and marks `dirty=true`. I/O happens in spawned jobs.

## 4. File Listing — `src/fs/mod.rs:1`

1. `list_dir(path)` reads `ReadDir` depth 1 (no recursive walk for speed).
2. For each entry: `metadata()` → `Entry { name, is_dir, size, mtime, ext }`.
3. Sort: dirs first, then alphabetical case-insensitive.
4. Filter: if `!config.show_hidden`, skip dotfiles.
5. Store in `App.files: Vec<Entry>`; selection index `App.selected: usize`.

**Edge:** Symlink → `read_link` depth ≤10, else show as broken.

## 5. Preview Pipeline — Step by Step

### Step 1 — Detect

- `mime_guess::from_path` gives `image/png`, `application/pdf`, etc.
- `infer::get(&magic_bytes)` confirms (e.g., `89 50 4E 47` → PNG regardless of ext).

### Step 2 — Route

- `router.route(path)` scores each handler's `priority()` → highest wins.
- Example: `report.pdf` → PdfHandler 95 > Text 10 → Pdf.

### Step 3 — Cache Check

```rust
let key = Cache::key(&path, area, HANDLER_VERSION);
if let Some(cached) = cache.get(&key).await { return cached; } // <1ms mem hit
if let Some(disk) = cache.get_disk(&key).await { return disk; } // <30ms disk hit
```

Key includes `area` size so resize invalidates correctly.

### Step 4 — Decode (spawn_blocking)

```rust
let handler = router.route(&path);
let ctx = PreviewCtx { path, area, cache_dir, config, cancel };
let job = tokio::task::spawn_blocking(move || handler.preview_blocking(ctx));
// On UI thread, show PreviewState::Loading with spinner
```

Decode runs on pool thread 1 or 2; UI remains 60 FPS.

### Step 5 — Render

- Handler returns `PreviewResult::Image/Text/Table/Audio`.
- `ui/preview_pane.rs` matches result → calls `term::graphics::render_image` or draws `Paragraph`/`Table`.
- For image fallback (no Kitty/Sixel): half-block rendering — two vertical pixels per cell.

### Step 6 — Cache Write

```rust
cache.put(key, result.clone()).await; // mem LRU + spawn_blocking disk write
```

Never blocks UI; disk write is fire-and-forget.

### Step 7 — Cancellation

If user scrolls fast `j` x10, prior 9 jobs are `abort()`ed; only 10th completes. No wasted CPU.

## 6. Handler Internals — What Happens per Format

### 6.1 Image (`image.png`, `logo.svg`)

```
image::open(path) → DynamicImage
  → if svg: usvg::Tree + resvg::render → RgbaImage
  → resize to fit area*2 via Lanczos3
  → term::graphics encode (Kitty base64 chunks)
```

SVG path distinct but converges to same `RgbaImage`.

### 6.2 PDF (`report.pdf`)

```
lopdf::Document::load → get_pages count
  → pdf_extract::extract_text(path) → first 2 pages text
  → #[cfg(pdf-raster)] mupdf::Document::open → page(0) → pixmap(150dpi) → DynamicImage
  → return split: Text pane + Image pane (if raster feature)
```

Without raster feature: text + metadata only (page count, author via `doc.trailer`).

### 6.3 DOCX/XLSX/PPTX

```
docx-rs: read_docx → document.body.children → map Paragraph/Run/Table → Line
calamine: open_workbook_auto → worksheet_range_at(0) → rows → TableData
pptx-rs: slides → title + bullets per slide → paginated Text
```

All from zip parsing, no external process.

### 6.4 Text/CSV/Markdown

```
content_inspector → if binary → Error
else encoding_rs decode → syntect highlight (per ext .rs/.py etc.)
if csv: csv::Reader → rows → Table
if md: pulldown_cmark → events → styled Lines
limit 5000 lines, else truncate + hint "… +1200 lines"
```

### 6.5 Audio (`song.mp3`)

```
lofty::read_from_path → tags (title, artist, album, duration)
symphonia::Probe → decode 30s → waveform Vec<u8> downsampled to 80
rodio::Sink on Space: symphonia decoder piped to rodio OutputStream
```

### 6.6 Video (`demo.mp4`)

```
#[cfg(feature="video")]:
  ffmpeg_next::format::input → metadata (duration, resolution, fps, codec)
  → seek to 10% duration → decode one frame → RgbImage → Image result
#[cfg(not(feature="video"))]:
  mp4::Mp4Reader header → metadata only + hint to rebuild with --features video
```

## 7. Terminal Rendering — `src/term/graphics.rs:1`

1. `detect()` probes env (`KITTY_WINDOW_ID`, `TERM_PROGRAM == iTerm.app`, `TERM == xterm-256color`) + queries `CSI ? 1;2 S` for Sixel.
2. `render_image`:
   - Kitty: `ESC _ G a=T,f=32,s=w,v=h,c=w,h=h,m=0;BASE64 ESC \` chunked 4096.
   - Sixel: `ESC P q "1;1;w;h ... ESC \`.
   - iTerm2: `ESC ] 1337 ; File=inline=1;width=w;height=h:BASE64 BEL`.
   - Fallback: half-block `▀` with `style.fg = top_pixel, style.bg = bottom_pixel` per `Buffer` cell. Doubles vertical resolution.

**Scroll performance:** Image escapes are written into Ratatui `Buffer`; Ratatui diffs and only flushes changed cells → minimal bandwidth.

## 8. State Machine — `src/app.rs:1`

```
State: files, selected, preview: Loading|Ready(Result)|Error, mode, search_query

Transitions:
  Normal + j/k         → selected +=1/-1, preview=Loading, spawn job
  Normal + /           → mode=Search, query=""
  Search + char        → query push, filter files via nucleo-matcher, selected=0
  Search + Esc         → mode=Normal, clear filter
  Normal + Enter(dir)  → path=selected.path, files=list_dir(path), selected=0
  Normal + Enter(file) → mode=FullscreenPreview
  Normal + f           → mode=FullscreenPreview toggle
  Normal + q           → break loop, restore terminal
  Any + ?              → mode=Help overlay
```

## 9. Error Flows — Never Crash on Bad File

```
File read error → PreviewResult::Error{msg: "permission denied", fallback: None}
Decode error (corrupt png) → Error + fallback Text showing hex dump of first 512B
Too large → Error + "Press Enter to force preview (may be slow)"
Unsupported (e.g., .exe) → Error + metadata panel (size, mtime, mime)
Panic in handler (bug) → catch_unwind → Error "handler panicked, file a bug"
```

All errors are displayed in preview pane with `Style::fg(Color::Red)` + hint.

## 10. Cache Invalidation — `src/cache/key.rs:1`

Key = `SHA256("{canonical_path}:{mtime_secs}:{size}:{area_w}x{area_h}:{handler_version}")`
- File edited → mtime changes → key miss → re-decode.
- Terminal resized → area changes → key miss → re-resize.
- Handler updated (bump `HANDLER_VERSION`) → all caches miss once.

Disk eviction: `evict_lru_disk()` lists `thumbs/*.png` sorted by mtime, deletes oldest until `total < 500MB`.

## 11. Performance Path — Hot Path Analysis

```
Hot path: key press → handle_key (0.1ms) → cache hit (0.5ms mem) → render (2ms)
Cold image: spawn_blocking decode (200ms) + resize (30ms) + cache write async
Worst case: 100MB pdf raster 150dpi (600ms) → but cancellable, shows text first (50ms)
Idle: event_stream parked, 0% CPU
```

Profiled with `tracing::instrument` spans; `debug.log` shows per-handler timings.

## 12. Data Flow Diagram (ASCII)

```
                    ┌─────────────┐
                    │  Filesystem │
                    └──────┬──────┘
                           │ list_dir
                    ┌──────▼──────┐
               ┌────┤   App State ├─────┐
               │    └──────┬──────┘     │
               │           │ route      │
               │    ┌──────▼──────┐     │
               │    │   Router    │     │
               │    └──────┬──────┘     │
               │     ┌─────┼──────┐     │
               │  Image  Pdf  Audio …   │
               │     └─────┼──────┘     │
               │    ┌──────▼──────┐     │
               └───►│    Cache    │◄────┘
                    └──────┬──────┘
                           │ hit/miss
                    ┌──────▼──────┐
                    │ Term Render │
                    └──────┬──────┘
                           │ escape seq
                    ┌──────▼──────┐
                    │  Terminal   │
                    └─────────────┘
```

## 13. Minimal Working Example

```powershell
cargo run -- ./fixtures
# fixtures contains: sample.png, demo.pdf, sales.xlsx, song.mp3, notes.md
# Navigate j/k, see each handler live
```

This is the **complete working loop** — every file goes through Detect → Route → Cache → Decode → Render → Cache, all pure Rust, all cancellable, all keyboard-driven.

# BACKEND.md — Preview Engine & Core Backend (Pure Rust)

> Backend = Preview Router + Handlers + Cache + FS + Terminal. No HTTP server; backend is the library that powers the TUI.
> **Review fixes:** centralized 5s timeout, quantized cache key, `(num_cpus/2).clamp` pool, `mupdf` → `pdfium-render`, `pptx-rs` → `zip+quick-xml`, `memmap2`, panic=unwind preserved.

## 1. Backend Overview

```
┌──────────────────────────────────────────────────────────────┐
│ Backend Crate (library)                                      │
│  src/preview/  ── Router + 8 Handlers (async, timeout-wrapped)│
│  src/cache/    ── Two-tier LRU + Disk (quantized keys)       │
│  src/fs/       ── Walk + Metadata + Watcher (optional)       │
│  src/term/     ── Capabilities + Graphics Encoding            │
│  src/config/   ── TOMLConfig                                  │
│  src/error/    ── Typed Errors                               │
└──────────────────────────────────────────────────────────────┘
```

All sync API is `async` via Tokio (trimmed features `rt, rt-multi-thread, macros, time, sync, fs`); handlers are `Send + Sync + 'static` dyn objects. `panic="unwind"` kept so `catch_unwind` works (see SECURITY.md §5).

## 2. Preview Router — `src/preview/router.rs:1`

### 2.1 Trait

```rust
#[async_trait::async_trait]
pub trait PreviewHandler: Send + Sync {
    fn name(&self) -> &'static str; // "image", "pdf", "docx"
    fn priority(&self, path: &Path, mime: &str, magic: &[u8]) -> u8; // 0-255
    async fn preview(&self, ctx: PreviewCtx) -> Result<PreviewResult, PreviewError>;
    fn file_size_limit(&self) -> u64; // 50MB image, 100MB pdf
}

pub struct PreviewCtx {
    pub path: PathBuf,
    pub area: Rect, // quantized before cache key
    pub cache_dir: PathBuf,
    pub config: Arc<Config>,
    pub cancel: CancellationToken,
}

pub enum PreviewResult {
    Text { lines: Vec<Line<'static>>, title: String, meta: FileMeta },
    Table { rows: Vec<Vec<String>>, headers: Vec<String>, meta: FileMeta },
    Image { rgba: DynamicImage, meta: FileMeta }, // rendered via term::graphics
    Audio { meta: AudioMeta, waveform: Vec<u8>, duration: Duration },
    Archive { entries: Vec<ArchiveEntry>, meta: FileMeta }, // NEW: zip/tar listing
    Directory { entries: Vec<EntrySummary> },
    Error { msg: String, fallback: Option<Box<PreviewResult>> },
}
```

### 2.2 Router Logic + Centralized Timeout — `src/preview/router.rs:40`

```rust
pub struct Router { handlers: Vec<Arc<dyn PreviewHandler>> }

impl Router {
    pub fn route(&self, path: &Path) -> Arc<dyn PreviewHandler> {
        let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
        let magic = read_magic(path, 512);
        let mut best = (0u8, 0usize);
        for (i, h) in self.handlers.iter().enumerate() {
            let p = h.priority(path, &mime, &magic);
            if p > best.0 { best = (p, i); }
        }
        self.handlers[best.1].clone()
    }

    /// Centralized dispatch — timeout enforced ONCE here, not per handler.
    /// Fixes review: 5s wall-clock timeout was mentioned per handler but not in trait.
    pub async fn dispatch(&self, ctx: PreviewCtx) -> PreviewResult {
        let handler = self.route(&ctx.path);
        let fut = tokio::task::spawn_blocking({
            let ctx = ctx.clone();
            let handler = handler.clone();
            move || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // handlers may use blocking APIs; run with block_in_place guard
                    tokio::runtime::Handle::try_current()
                        .map(|h| h.block_on(handler.preview(ctx.clone())))
                        .unwrap_or_else(|_| futures::executor::block_on(handler.preview(ctx.clone())))
                }))
            }
        });
        // 5s wall-clock enforced centrally — cannot be forgotten on new handlers
        match tokio::time::timeout(Duration::from_secs(5), fut).await {
            Ok(Ok(Ok(Ok(result)))) => result,
            Ok(Ok(Ok(Err(e)))) => PreviewResult::Error { msg: e.to_string(), fallback: None },
            Ok(Ok(Err(_))) => PreviewResult::Error { msg: "handler panicked (malformed file)".into(), fallback: None },
            Ok(Err(_)) => PreviewResult::Error { msg: "task join error".into(), fallback: None },
            Err(_) => PreviewResult::Error { msg: "preview timed out (5s)".into(), fallback: None },
        }
    }
}
```

**Why centralized:** Per-handler `timeout()` is easy to forget when adding a new handler. One wrapper in `dispatch()` guarantees every handler is bounded.

**Why `catch_unwind` works:** `Cargo.toml` keeps `panic="unwind"` (removed `abort`), so `catch_unwind` can catch malformed-file panics and degrade to `Error` pane instead of crashing.

Priority table:

| Handler | Ext/MIME | Magic | Priority |
|---|---|---|---|
| Image | png/jpg/gif/webp/bmp/svg | 89 50 4E 47 etc. | 100 |
| Archive | zip/tar/tgz | PK / `ustar` | 96 |
| Pdf | pdf | %PDF | 95 |
| OfficeDocx | docx | PK + word/ | 90 |
| OfficeXlsx | xlsx/xls | PK + xl/ | 90 |
| OfficePptx | pptx | PK + ppt/ | 90 |
| Video | mp4/mkv/webm | ftyp/ftyp | 85 |
| Audio | mp3/flac/wav | ID3/ff… | 85 |
| Text/Csv | txt/csv/md/rs | text/* | 70 |
| Fallback | * | — | 10 |

Tie-break by registration order.

### 2.3 Handler Registry — `src/preview/mod.rs:1`

```rust
pub fn build_router(config: &Config) -> Router {
    let mut h: Vec<Arc<dyn PreviewHandler>> = vec![
        Arc::new(ImageHandler::new(config.image.clone())),
        Arc::new(ArchiveHandler::new()), // NEW: zip/tar via existing zip crate
        Arc::new(TextHandler::new(config.text.clone())),
        Arc::new(CsvHandler::new()),
        Arc::new(PdfHandler::new(config.pdf.clone())),
        Arc::new(OfficeHandler::new()), // now zip+quick-xml for pptx, no pptx-rs
        Arc::new(AudioHandler::new()),
    ];
    #[cfg(feature = "video")]
    h.push(Arc::new(VideoHandler::new()));
    #[cfg(not(feature = "video"))]
    h.push(Arc::new(VideoMetaHandler::new())); // metadata only via mp4 crate
    Router::new(h)
}
```

## 3. Handler Deep Dive

### 3.1 ImageHandler — `src/preview/image.rs:1`

```rust
impl PreviewHandler for ImageHandler {
    fn preview(&self, ctx: PreviewCtx) -> Result<PreviewResult> {
        // 1. Check size limit
        // 2. Check cache via cache::get(quantized_key)
        // 3. spawn_blocking: image::open(path) or resvg::render for svg
        // 4. Resize to fit quantized_area: imageops::resize(w*2, h*2, Lanczos3)
        // 5. cache::put(key, resized)
        // 6. Return Image{rgba, meta}
    }
}
```
- Cache key includes **quantized** `width/height` (rounded to 8 cols × 4 rows) so one-pixel resize doesn't churn.
- SVG via `usvg::Tree::from_str` + `resvg::render`.
- Limit: 50MB file, 10000x10000 max, else return `Error + meta fallback`.

### 3.2 TextHandler — `src/preview/text.rs:1`

- Detect binary via `content_inspector::inspect` → if binary, return `Error("binary")`.
- For files >1MB, use `memmap2::Mmap::map(&file)` to avoid `read()` copy (NEW: per review, avoids churn on large log files over SSH).
- Detect encoding via `encoding_rs`, convert to UTF-8.
- If `ext == .md` → `pulldown_cmark` parse → styled `Line`.
- Else → `syntect::parsing::SyntaxSet + ThemeSet` highlight per file extension, limit 5000 lines / 2MB.
- Returns `Text`.

### 3.3 CsvHandler — `src/preview/csv.rs:1`

- `csv::Reader::from_path` streaming, read first 100 rows + header.
- Detect delimiter via sniffing `,;|\t`.
- Return `Table { headers, rows }` rendered via `comfy-table` → Ratatui `Table` widget.

### 3.4 ArchiveHandler — `src/preview/archive.rs:1` (NEW)

- `zip::ZipArchive::new(file)` or `tar::Archive::new(flate2::read::GzDecoder)` → iterate entries → `ArchiveEntry { name, size, compressed_size, ratio }`.
- Return `Table` with columns `Name | Size | Packed | Ratio`. No extraction.
- Limit zip entries 10k, abort decompression bomb (>100:1 ratio).

### 3.5 PdfHandler — `src/preview/pdf.rs:1`

```rust
// Phase 1: lopdf
let doc = lopdf::Document::load(path)?;
let text = pdf_extract::extract_text(path)?; // first 2 pages
let pages = doc.get_pages().len();
// Phase 2 (feature pdf-raster): pdfium-render (Apache-2.0, NOT mupdf AGPL)
#[cfg(feature = "pdf-raster")]
let pdfium = Pdfium::default();
let doc = pdfium.load_pdf_from_file(path, None)?;
let page = doc.pages().get(0)?;
let pixmap = page.render_with_config(&PdfRenderConfig::new().set_target_width(800))?;
```
- v1 shows `Text (page 1)` + optional `Image` pane split if `pdf-raster` feature (pdfium).
- **mupdf removed:** was `mupdf::Document::open` — AGPL-3.0, replaced with `pdfium-render` (Apache-2.0).
- Full raster pagination via `n/p` keys (next/prev page) in `FullscreenPreview` mode.

### 3.6 OfficeHandler — `src/preview/office.rs:1`

- DOCX: `docx_rs::read_docx(path)?.document.children` → iterate paragraphs/tables → `Line`.
- XLSX: `calamine::open_workbook_auto(path)?.worksheet_range_at(0)` → first sheet rows → `Table`, sheet picker via `Tab`.
- PPTX: **NEW in-house via `zip` + `quick-xml`** — open `ppt/slides/slide*.xml`, `quick_xml::Reader` extracts `<a:t>` text runs per slide → paginated `Text`. Replaces `pptx_rs` (abandoned, 1 release, no repo).
- All via `zip` under the hood; no LO.

### 3.7 AudioHandler — `src/preview/audio.rs:1`

- `lofty::read_from_path` → `AudioMeta { title, artist, album, duration, bitrate, sample_rate }`.
- Waveform: `symphonia::probe` → decode first 30s → downsample `waveform: Vec<u8>` (0-255) for `Sparkline`.
- Playback: `rodio::OutputStream + Sink::append(decoder)`, controls `Space` play/pause, `s` stop. Sink stored in `App` state.

### 3.8 VideoHandler — `src/preview/video.rs:1`

- `#[cfg(feature="video")]`: `ffmpeg_next::format::input(path)?.stream(0)` → metadata (duration, codec, res, fps), seek to 10% → `frame -> image::RgbImage` thumbnail → `Image`.
- `#[cfg(not(feature="video"))]`: `mp4::Mp4Reader::read_header` → metadata + placeholder `Text("Video — build with --features video for thumbnails")`.

## 4. Cache Backend — `src/cache/mod.rs:1`

```rust
pub struct Cache {
    mem: Mutex<LruCache<String, Arc<PreviewResult>>>, // 100 entries
    dir: PathBuf,
    max_disk_bytes: u64, // 500MB
    // worker pool size computed once at startup: (num_cpus/2).clamp(2,6)
}
impl Cache {
    /// Quantized area: round w to 8 cols, h to 4 rows to avoid churn on pixel resize.
    fn quantize(area: Rect) -> Rect {
        Rect { x: area.x, y: area.y, width: (area.width / 8) * 8, height: (area.height / 4) * 4 }
    }
    pub fn key(path: &Path, area: Rect, version: u8) -> String {
        let q = Self::quantize(area);
        let meta = fs::metadata(path).unwrap();
        let input = format!("{}:{}:{}:{}:{}:{}", path.display(), meta.mtime(), meta.len(), q.width, q.height, version);
        hex::encode(Sha256::digest(input))
    }
    pub async fn get(&self, key: &str) -> Option<Arc<PreviewResult>>;
    pub async fn put(&self, key: String, val: Arc<PreviewResult>);
    pub fn evict_lru_disk(&self); // remove oldest files until under cap
}
```

- Fix: Area quantized (8×4) — one-pixel resize no longer generates new key → no re-decode storm.
- Fix: Worker pool `size = (num_cpus::get()/2).clamp(2,6)` via `num_cpus` crate, configurable `cache.worker_threads` in config.toml.
- Mem hit: <1ms. Disk hit: read png → decode → <30ms. Write: `spawn_blocking` write png via `image::save`, never blocks UI. Eviction runs on `put` if `dir size > cap`.

## 5. Filesystem Backend — `src/fs/mod.rs:1` (+ `src/fs/du.rs:1`)

```rust
pub struct Entry { pub path: PathBuf, pub name: String, pub is_dir: bool, pub size: u64, pub mtime: SystemTime, pub ext: String, pub git_status: Option<GitStatus> }
pub fn list_dir(path: &Path, show_hidden: bool) -> Result<Vec<Entry>> {
    // walkdir depth 1, sorted dirs first + alpha, filter hidden if !show_hidden
    // if feature git: gix::status to annotate modified/untracked per entry
}
pub fn file_meta(path: &Path) -> FileMeta { /* size, mtime, perms, mime, magic, exif */ }
pub fn du(path: &Path) -> u64 { /* async walk + sum, cached, triggered by D key */ }
```

- Sorting: `entries.sort_by(|a,b| match (a.is_dir,b.is_dir){ (true,false)=>Less, (false,true)=>Greater, _=>a.name.cmp(&b.name)})`.
- Hidden toggle `h` key. Git badges (`git` feature, `gix` crate) — `M` modified, `?` untracked.
- `du` on demand: `D` key computes directory size async, cached.
- Symlink: follow depth ≤10, else show as `symlink -> target` text.
- Watcher: `src/fs/watcher.rs` compiled only with `cfg(feature="watch")` — `notify` is optional, not in default deps.

## 6. Terminal Backend — `src/term/*:1`

- `capabilities.rs`: `Capability { kitty: bool, sixel: bool, iterm2: bool, truecolor: bool, unicode: bool }`.
- `graphics.rs`: `fn render_image(img: &DynamicImage, area: Rect, cap: &Capability, buf: &mut Buffer)` writes escape sequences into Ratatui `Buffer` via `Cell`.
- For fallback half-block, writes `▄` with `fg = top pixel, bg = bottom pixel`.

## 7. Config Backend — `src/config.rs:1`

```rust
#[derive(Deserialize)] pub struct Config {
    pub general: General { theme: String, show_hidden: bool, preview_delay_ms: u64 },
    pub cache: CacheCfg { max_disk_mb: u64, mem_entries: usize, worker_threads: usize },
    pub preview: PreviewCfg { max_image_mb: u64, max_pdf_pages: usize },
    pub keys: KeyMap, // customizable
}
```
Loaded from `~/.config/tui-preview/config.toml` else defaults. `directories::ProjectDirs::from("com","tui-preview","tui-preview")`. `worker_threads` defaults to `(num_cpus/2).clamp(2,6)`.

## 8. Error Handling — `src/error.rs:1`

```rust
#[derive(thiserror::Error, Debug)]
pub enum PreviewError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("unsupported: {0}")] Unsupported(String),
    #[error("too large: {0} bytes > limit {1}")] TooLarge(u64, u64),
    #[error("decode: {0}")] Decode(String),
    #[error("cancelled")] Cancelled,
    #[error("timeout")] Timeout,
}
```

All handlers map to `PreviewResult::Error` for UI display, never bubble panic. Timeouts bubble as `Timeout` via router's centralized wrapper.

## 9. Async & Cancellation — `src/app.rs:80`

```rust
let (tx, mut rx) = mpsc::channel(8);
let mut jobs: JoinSet<(String, PreviewResult)> = JoinSet::new();
let mut current_abort: Option<AbortHandle> = None;
// On selection change:
if let Some(h) = current_abort.take() { h.abort(); }
let handle = jobs.spawn(async move { (key, router.dispatch(ctx).await) });
current_abort = Some(handle);
```

`dispatch()` already wraps handler in 5s timeout + catch_unwind — `app.rs` needs no extra timeout logic.

## 10. Testing Backend

- Unit: `cargo test --lib preview::image::tests` — mock small png/svg, verify quantized resize + cache key.
- Integration: `tests/preview_golden` with fixture files `fixtures/{image.pdf, docx, xlsx, mp3}`.
- Snapshot: `insta` for text handler highlight output.
- Supply: `cargo deny check` (bans AGPL) + `cargo audit` in CI from day one.

This backend is **pure Rust, async, typed, cancellable, cached (quantized), timeout-bounded, and panic-safe** — review fixes applied.

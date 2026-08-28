# ARCHITECTURE.md — tui-preview (Pure Rust) Professional Grade Architecture

> Goal: Lightweight, Fast, Efficient, Working, Rich-Featured terminal file previewer for developers/power users. Zero shell-out in core, single binary, async, cached, keyboard-driven.

## 1. Vision Summary

An orchestrator TUI that **previews instead of replaces** native apps. Fast triage over SSH, no GUI needed. Heavy lifting via pure Rust crates, not external binaries. Inspired by Yazi/Ranger + `chafa` + `bat` + `mpv` but unified and pure Rust.

```
Native GUI advantages: smooth video, perfect Office layout, GPU zoom
    ↓
tui-preview advantages: instant, keyboard, SSH, scriptable, cached, 8-15 MB binary, <50ms cached preview
```

## 2. High-Level System Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                        tui-preview Binary                       │
│  ┌─────────────┐  ┌──────────────────┐  ┌────────────────────┐  │
│  │  Frontend   │◄─┤   App Core       │◄─┤  Backend Engine    │  │
│  │  Ratatui UI │  │  State + Events  │  │  Preview Router    │  │
│  │  crossterm  │  │  Tokio Runtime   │  │  Handlers (async)  │  │
│  └──────┬──────┘  └────────┬─────────┘  └─────────┬──────────┘  │
│         │                  │                      │             │
│  ┌──────▼──────┐    ┌──────▼──────┐        ┌─────▼─────┐       │
│  │ Term Layer  │    │ Cache Layer │        │ FS Layer  │       │
│  │ Graphics    │    │ LRU + Disk  │        │ Walk +    │       │
│  │ Input       │    │ SHA Cache   │        │ Watch     │       │
│  └─────────────┘    └─────────────┘        └───────────┘       │
└─────────────────────────────────────────────────────────────────┘
         ▲                   │                      ▲
         │                   ▼                      │
    Terminal (Kitty/Sixel/ASCII)  ~/.cache/tui-preview  Filesystem
```

## 3. Layered Architecture (6 Layers)

### Layer 1 — Presentation ( `src/ui/*` )
- **Ratatui 0.29** immediate mode, double-buffered.
- Layout: `Horizontal [30% FileList | 70% Preview]` + `Footer StatusBar (1 line)` + `Overlay Modal`.
- Widgets: `FileList (List)`, `PreviewPane (Paragraph/Table/Image)`, `StatusBar`, `HelpModal`, `SearchBar`.
- 60 FPS tick, but render only on state change (dirty flag) to save CPU.

### Layer 2 — Application Core ( `src/app.rs`, `src/event.rs`, `src/state.rs` )
- `App { files: Vec<Entry>, selected: usize, preview: PreviewState, mode: Mode, config, cache }`
- `Mode = Normal | Search | Help | FullscreenPreview`
- Event loop: `crossterm::event::EventStream` + `tokio::select!` (input, tick, worker results).
- State machine: Navigation never blocks; preview requests are async messages.

### Layer 3 — Preview Router & Handlers ( `src/preview/*` )
```rust
trait PreviewHandler: Send + Sync {
    fn can_handle(&self, path: &Path, mime: &str) -> bool; // priority score 0-100
    async fn preview(&self, ctx: PreviewCtx) -> Result<PreviewResult, PreviewError>;
    fn name(&self) -> &'static str;
}
struct PreviewCtx { path: PathBuf, area: Rect, cache_dir: PathBuf, config: Arc<Config> }
enum PreviewResult { Text{lines: Vec<Line>, meta: Meta}, Table{table: TableData}, Image{rgba: DynamicImage}, Audio{meta: AudioMeta, waveform: Vec<u8>}, Error{msg: String} }
```
- Router picks highest priority handler via `mime_guess` + magic bytes (first 512 bytes).
- Pipeline: `Detect -> Check Cache -> Decode (async pool) -> Render -> Cache Write -> UI Update`.

### Layer 4 — Terminal Abstraction ( `src/term/*` )
- `term::capabilities::detect()` at startup: reads `TERM`, `TERM_PROGRAM`, `COLORTERM`, queries `DA1` for Sixel/Kitty.
- `term::graphics::render_image(img, area, cap)` — dispatch:
  1. Kitty Graphics Protocol (`\x1b_Ga=T,f=32,s=w,v=h...`)
  2. Sixel (`\x1bPq...`)
  3. iTerm2 Inline (`\x1b]1337;File=...`)
  4. Fallback: Half-block `▀` + truecolor (2 pixels per cell) via `image::imageops` resize.
- Input: crossterm key parser, vim-style `j/k, g/G, /, n, p, f, m, q, ?, Enter, Esc`.

### Layer 5 — Cache & Performance ( `src/cache/*` )
- Two-tier: `Memory LRU (100 entries, ~50MB)` + `Disk (~/.cache/tui-preview/thumbs/<sha256>.png, 500MB cap)`.
- Key: `sha256(canonical_path + mtime_secs + size + quantized_area + handler_version)` — area quantized to 8 cols × 4 rows (see §5.1) to avoid churn on pixel resize.
- Async worker pool: `tokio::task::spawn_blocking` sized `(num_cpus/2).clamp(2,6)` via `num_cpus` crate (configurable `cache.worker_threads`), `JoinSet`, cancellation on selection change via `AbortHandle`.
- Centralized timeout: router wraps every handler dispatch in `tokio::time::timeout(5s, ...)` — per-handler timeouts not duplicated.
- Limits guard heavy files: `>50MB image, >100MB pdf/video -> Meta-only + press Enter to force`.

#### 5.1 Cache Key Quantization (fixes churn)
`Rect {width,height}` rounded down to nearest `8 cols × 4 rows` before hashing. One-pixel resize no longer generates new key → no re-decode storm.

### Layer 6 — Filesystem ( `src/fs/*` )
- `walkdir` with `.gitignore` respect, `.tui-ignore`, sorted `dirs first, alpha`.
- `notify` crate optional file watcher for live reload — **feature-gated `watch` (`--features watch`)**; absent from default deps to keep binary lean. When feature off, `fs/watcher.rs` is not compiled.
- Safe path handling: `canonicalize` + symlink depth limit 10.

## 4. Crate Module Map

```
src/
├─ main.rs              # tokio::main, setup terminal, run App
├─ app.rs               # App struct, update() reducer
├─ event.rs             # Event enum, input mapping
├─ config.rs            # load ~/.config/tui-preview/config.toml
├─ error.rs             # thiserror enums
├─ fs/
│  ├─ mod.rs            # list_dir, file entry types
│  └─ watcher.rs        # notify integration — only with `watch` feature (cfg(feature="watch"))
├─ preview/
│  ├─ mod.rs            # trait + router (centralized timeout, quantized cache key)
│  ├─ image.rs          # image + resvg
│  ├─ text.rs           # syntect + csv + markdown (memmap2 for large files)
│  ├─ archive.rs        # zip/tar/tar.gz/7z listing (new, cheap)
│  ├─ pdf.rs            # lopdf + pdf-extract + pdfium-render (feature pdf-raster, NOT mupdf AGPL)
│  ├─ office.rs         # docx-rs + calamine + zip+quick-xml pptx (pptx-rs removed: abandoned)
│  ├─ audio.rs          # symphonia + lofty + rodio
│  ├─ video.rs          # ffmpeg-next (feature) / mp4 header
│  └─ meta.rs           # file metadata extractor + EXIF + du
├─ term/
│  ├─ mod.rs
│  ├─ capabilities.rs   # detect Kitty/Sixel
│  └─ graphics.rs       # image -> terminal escape
├─ cache/
│  ├─ mod.rs            # LRU + disk
│  └─ key.rs            # hash logic
└─ ui/
   ├─ mod.rs
   ├─ layout.rs         # Ratatui layout
   ├─ file_list.rs
   ├─ preview_pane.rs
   └─ status.rs
```

## 5. Data Flow — Preview Request Lifecycle

```
User presses j (down)
  → event.rs maps to Action::SelectNext
  → app.rs: selected+=1, preview=Loading, spawn async job
  → preview/router.rs: pick handler (e.g., ImageHandler)
  → cache::get(key) ? Hit→ return Image : Miss→ spawn_blocking decode
  → handler decodes (image::open, resvg, etc.)
  → cache::put(key, rgba)
  → channel -> app.rs receives PreviewReady
  → ui re-renders preview pane via term::graphics
  → status bar shows "image 4032x3024 (cached 12ms)"
```

All decodes off main thread; UI never blocks >16ms.

## 6. Concurrency Model

- **Main thread:** Tokio multi_thread runtime (trimmed features `rt, rt-multi-thread, macros, time, sync, fs`) + crossterm event stream.
- **Blocking pool:** Sized `(num_cpus::get()/2).clamp(2,6)` via `num_cpus`, configurable `cache.worker_threads` in config.toml; `tokio::task::spawn_blocking` + `JoinSet`.
- **Centralized timeout:** `tokio::time::timeout(Duration::from_secs(5), handler.preview(ctx))` in `src/preview/mod.rs:router dispatch` — single enforcement point, cannot be forgotten per handler.
- **Unwind preserved:** `Cargo.toml` keeps `panic="unwind"` (default) so `catch_unwind` inside `spawn_blocking` works; `panic="abort"` removed (was breaking SECURITY.md §5).
- **Large text zero-copy:** `text.rs` uses `memmap2::Mmap` for read-only mapped access on files >1MB, avoiding read+copy.
- **Cancellation:** Each preview job has `AbortHandle`; new selection aborts previous if not yet done.
- **Backpressure:** Channel size 8, drop oldest if flooded (fast scrolling).

## 7. Error & Fallback Strategy

```
Decode succeeds → show rich preview
Decode fails → show Error pane with reason + fallback text preview (first 4KB)
Unsupported terminal → fallback to half-block, never crash
Missing feature (video without ffmpeg) → show metadata + "build with --features video"
Large file → show metadata + "Enter to force preview"
```

Never panic on user file; all handlers return `Result`, UI shows degraded view.

## 8. Memory & Performance Budgets

| Component | Idle | Active Preview | Cap |
|---|---|---|---|
| App + Ratatui | 8 MB | 12 MB | — |
| Memory LRU | 0 | up to 50 MB | 100 entries |
| Disk cache | — | 500 MB | auto-evict LRU |
| Single image decode | — | peak 2× image size | 50 MB limit |
| PDF raster 150dpi | — | ~20 MB/page | 1 page v1 |
| XLSX parse | — | streaming | 100 MB file cap |

Goal: Startup <80ms, navigation <16ms, cached preview <30ms, cold image <300ms.

## 9. Security Boundaries

- No `unsafe` except in `ffmpeg-next`/`pdfium-render` bindings (feature-gated, audited). `mupdf` (AGPL-3.0) removed entirely — `pdfium-render` is Apache-2.0.
- No `Command::new`; deny via `clippy.toml: disallowed-methods = ["std::process::Command::new"]`.
- Lock file: `Cargo.lock` COMMITTED (binary crate, supply-chain audit) — `.gitignore` fixed not to exclude it.
- Keep `panic="unwind"` so `catch_unwind` works (see SECURITY.md §5).
- Limit decompression bombs: SVG depth 100, zip entries 10k, image dimensions 10000x10000, `cargo deny` bans GPL/AGPL crates.
- Sandboxed parsing: all file reads via `std::fs::read` / `memmap2` + size check before decode.

## 10. Config & Extensibility

- `config.toml` TOML: keybinds, preview limits, cache size, theme (dark/light), default handler priorities.
- Plugin horizon: `preview` trait is dyn object — third-party handlers can be added via feature flags without core change.
- Future: Lua/Rhai scripting for custom previewers (post v1).

## 11. Build & Distribution Architecture

- Single static binary, `cargo build --release`, LTO + strip, `panic="unwind"` preserved.
- Tokio trimmed (`rt, rt-multi-thread, macros, time, sync, fs`) — no `full` (saves compile time + binary size).
- `Cargo.lock` committed; CI runs `cargo audit` + `cargo deny` on every push.
- Cross-compile via `cargo-xwin` / `cargo-zigbuild`.
- Release artifacts: `tui-preview-{linux-x64,macos-arm64,windows-x64}.tar.gz` + `cargo install tui-preview`.
- Feature flags keep base binary pure Rust (~10 MB) vs `full` (~25 MB with FFmpeg/pdfium).
- CI: `.github/workflows/ci.yml` from day one — `fmt`, `clippy -D warnings`, `test`, `audit`, `deny` — green pipeline before `src/` lands.

## 12. Alternatives Considered & Rejected

| Approach | Rejected Because |
|---|---|
| Shell out to `ffmpeg/soffice/pdftoppm` | Breaks pure Rust, Windows fragile, SSH issues, slow spawn |
| Python Textual | Heavy, not pure Rust, bad for headless |
| Electron/Tauri GUI | Not terminal, defeats purpose |
| Neovim plugin | Limited to Neovim users, not standalone |

This architecture delivers **lightweight UI + heavy but cached & async decode + pure Rust reliability** — the power design you requested.

## 13. References
- Ratatui: https://ratatui.rs
- viuer/Kitty protocol: https://sw.kovidgoyal.net/kitty/graphics-protocol/
- image crate: https://crates.io/crates/image
- symphonia: https://crates.io/crates/symphonia

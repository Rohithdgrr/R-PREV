# BACKEND.md — Preview Engine & Core Backend (Pure Rust)

> Backend = Preview Router + Handlers + Cache + FS + Terminal. No HTTP server; backend is the library that powers the TUI.

## 1. Backend Overview

```
┌──────────────────────────────────────────────────────────────┐
│ Backend Crate (library)                                      │
│  src/preview/  ── Router + 7 Handlers (async)                │
│  src/cache/    ── Two-tier LRU + Disk                        │
│  src/fs/       ── Walk + Metadata + Watcher                  │
│  src/term/     ── Capabilities + Graphics Encoding            │
│  src/config/   ── TOMLConfig                                  │
│  src/error/    ── Typed Errors                               │
└──────────────────────────────────────────────────────────────┘
```

All sync API is `async` via Tokio; handlers are `Send + Sync + 'static` dyn objects.

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
    pub area: Rect, // terminal area for sizing
    pub cache_dir: PathBuf,
    pub config: Arc<Config>,
    pub cancel: CancellationToken,
}

pub enum PreviewResult {
    Text { lines: Vec<Line<'static>>, title: String, meta: FileMeta },
    Table { rows: Vec<Vec<String>>, headers: Vec<String>, meta: FileMeta },
    Image { rgba: DynamicImage, meta: FileMeta }, // rendered via term::graphics
    Audio { meta: AudioMeta, waveform: Vec<u8>, duration: Duration },
    Directory { entries: Vec<EntrySummary> },
    Error { msg: String, fallback: Option<Box<PreviewResult>> },
}
```

### 2.2 Router Logic — `src/preview/router.rs:40`

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
}
```

Priority table:

| Handler | Ext/MIME | Magic | Priority |
|---|---|---|---|
| Image | png/jpg/gif/webp/bmp/svg | 89 50 4E 47 etc. | 100 |
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
        Arc::new(TextHandler::new(config.text.clone())),
        Arc::new(CsvHandler::new()),
        Arc::new(PdfHandler::new(config.pdf.clone())),
        Arc::new(OfficeHandler::new()),
        Arc::new(AudioHandler::new()),
    ];
    #[cfg(feature = "video")]
    h.push(Arc::new(VideoHandler::new()));
    #[cfg(not(feature = "video"))]
    h.push(Arc::new(VideoMetaHandler::new())); // metadata only
    Router::new(h)
}
```

## 3. Handler Deep Dive

### 3.1 ImageHandler — `src/preview/image.rs:1`

```rust
impl PreviewHandler for ImageHandler {
    fn preview(&self, ctx: PreviewCtx) -> Result<PreviewResult> {
        // 1. Check size limit
        // 2. Check cache via cache::get(key)
        // 3. spawn_blocking: image::open(path) or resvg::render for svg
        // 4. Resize to fit area: imageops::resize(w*2, h*2, Lanczos3)
        // 5. cache::put(key, resized)
        // 6. Return Image{rgba, meta}
    }
}
```
- Cache key includes `width/height` so resizing on terminal resize invalidates.
- SVG via `usvg::Tree::from_str` + `resvg::render`.
- Limit: 50MB file, 10000x10000 max, else return `Error + meta fallback`.

### 3.2 TextHandler — `src/preview/text.rs:1`

- Detect binary via `content_inspector::inspect` → if binary, return `Error("binary")`.
- Detect encoding via `encoding_rs`, convert to UTF-8.
- If `ext == .md` → `pulldown_cmark` parse → styled `Line`.
- Else → `syntect::parsing::SyntaxSet + ThemeSet` highlight per file extension, limit 5000 lines / 2MB.
- Returns `Text`.

### 3.3 CsvHandler — `src/preview/csv.rs:1`

- `csv::Reader::from_path` streaming, read first 100 rows + header.
- Detect delimiter via sniffing `,;|\t`.
- Return `Table { headers, rows }` rendered via `comfy-table` → Ratatui `Table` widget.

### 3.4 PdfHandler — `src/preview/pdf.rs:1`

```rust
// Phase 1: lopdf
let doc = lopdf::Document::load(path)?;
let text = pdf_extract::extract_text(path)?; // first 2 pages
let pages = doc.get_pages().len();
// Phase 2 (feature pdf-raster): mupdf
#[cfg(feature = "pdf-raster")]
let pixmap = mupdf::Document::open(path)?.page(0)?.to_pixmap(150.0)?;
```
- v1 shows `Text (page 1)` + optional `Image` pane split if raster feature.
- Full raster pagination via `n/p` keys (next/prev page) in `FullscreenPreview` mode.

### 3.5 OfficeHandler — `src/preview/office.rs:1`

- DOCX: `docx_rs::read_docx(path)?.document.children` → iterate paragraphs/tables → `Line`.
- XLSX: `calamine::open_workbook_auto(path)?.worksheet_range_at(0)` → first sheet rows → `Table`, sheet picker via `Tab`.
- PPTX: `pptx_rs` slide titles + notes → paginated `Text`.
- All via `zip` under the hood; no LO.

### 3.6 AudioHandler — `src/preview/audio.rs:1`

- `lofty::read_from_path` → `AudioMeta { title, artist, album, duration, bitrate, sample_rate }`.
- Waveform: `symphonia::probe` → decode first 30s → downsample `waveform: Vec<u8>` (0-255) for `Sparkline`.
- Playback: `rodio::OutputStream + Sink::append(decoder)`, controls `Space` play/pause, `s` stop. Sink stored in `App` state.

### 3.7 VideoHandler — `src/preview/video.rs:1`

- `#[cfg(feature="video")]`: `ffmpeg_next::format::input(path)?.stream(0)` → metadata (duration, codec, res, fps), seek to 10% → `frame -> image::RgbImage` thumbnail → `Image`.
- `#[cfg(not(feature="video"))]`: `mp4::Mp4Reader::read_header` → metadata + placeholder `Text("Video — build with --features video for thumbnails")`.

## 4. Cache Backend — `src/cache/mod.rs:1`

```rust
pub struct Cache {
    mem: Mutex<LruCache<String, Arc<PreviewResult>>>, // 100 entries
    dir: PathBuf,
    max_disk_bytes: u64, // 500MB
}
impl Cache {
    pub fn key(path: &Path, area: Rect, version: u8) -> String {
        let meta = fs::metadata(path).unwrap();
        let input = format!("{}:{}:{}:{}:{}", path.display(), meta.mtime(), meta.len(), area, version);
        hex::encode(Sha256::digest(input))
    }
    pub async fn get(&self, key: &str) -> Option<Arc<PreviewResult>>;
    pub async fn put(&self, key: String, val: Arc<PreviewResult>);
    pub fn evict_lru_disk(&self); // remove oldest files until under cap
}
```

- Mem hit: <1ms.
- Disk hit: read png → decode → <30ms.
- Write: `spawn_blocking` write png via `image::save`, never blocks UI.
- Eviction runs on `put` if `dir size > cap`.

## 5. Filesystem Backend — `src/fs/mod.rs:1`

```rust
pub struct Entry { pub path: PathBuf, pub name: String, pub is_dir: bool, pub size: u64, pub mtime: SystemTime, pub ext: String }
pub fn list_dir(path: &Path, show_hidden: bool) -> Result<Vec<Entry>> {
    // walkdir depth 1, sorted dirs first + alpha, filter hidden if !show_hidden
}
pub fn file_meta(path: &Path) -> FileMeta { /* size, mtime, perms, mime, magic */ }
```

- Sorting: `entries.sort_by(|a,b| match (a.is_dir,b.is_dir){ (true,false)=>Less, (false,true)=>Greater, _=>a.name.cmp(&b.name)})`.
- Hidden toggle `h` key.
- Symlink: follow depth ≤10, else show as `symlink -> target` text.

## 6. Terminal Backend — `src/term/*:1`

- `capabilities.rs`: `Capability { kitty: bool, sixel: bool, iterm2: bool, truecolor: bool, unicode: bool }`.
- `graphics.rs`: `fn render_image(img: &DynamicImage, area: Rect, cap: &Capability, buf: &mut Buffer)` writes escape sequences into Ratatui `Buffer` via `Cell`.
- For fallback half-block, writes `▄` with `fg = top pixel, bg = bottom pixel`.

## 7. Config Backend — `src/config.rs:1`

```rust
#[derive(Deserialize)] pub struct Config {
    pub general: General { theme: String, show_hidden: bool, preview_delay_ms: u64 },
    pub cache: CacheCfg { max_disk_mb: u64, mem_entries: usize },
    pub preview: PreviewCfg { max_image_mb: u64, max_pdf_pages: usize },
    pub keys: KeyMap, // customizable
}
```
Loaded from `~/.config/tui-preview/config.toml` else defaults. `directories::ProjectDirs::from("com","tui-preview","tui-preview")`.

## 8. Error Handling — `src/error.rs:1`

```rust
#[derive(thiserror::Error, Debug)]
pub enum PreviewError {
    #[error("io: {0}")] Io(#[from] std::io::Error),
    #[error("unsupported: {0}")] Unsupported(String),
    #[error("too large: {0} bytes > limit {1}")] TooLarge(u64, u64),
    #[error("decode: {0}")] Decode(String),
    #[error("cancelled")] Cancelled,
}
```

All handlers map to `PreviewResult::Error` for UI display, never bubble panic.

## 9. Async & Cancellation — `src/app.rs:80`

```rust
let (tx, mut rx) = mpsc::channel(8);
let mut jobs: JoinSet<(String, PreviewResult)> = JoinSet::new();
let mut current_abort: Option<AbortHandle> = None;
// On selection change:
if let Some(h) = current_abort.take() { h.abort(); }
let handle = jobs.spawn(async move { (key, handler.preview(ctx).await) });
current_abort = Some(handle);
```

## 10. Testing Backend

- Unit: `cargo test --lib preview::image::tests` — mock small png/svg, verify resize + cache key.
- Integration: `tests/preview_golden` with fixture files `fixtures/{image.pdf, docx, xlsx, mp3}`.
- Snapshot: `insta` for text handler highlight output.

This backend is **pure Rust, async, typed, cancellable, cached, and bounded** — production grade.

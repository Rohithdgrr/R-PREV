# TECH-STACK.md — Pure Rust Stack (Lightweight, Fast, Efficient)

> Constraint: **PURELY RUST**. No `std::process::Command` to `ffmpeg/soffice/pdftoppm` in core. Single binary via `cargo build --release`. All heavy lifting via crates.

## 1. Stack Philosophy

- **Zero shell-out core** → Windows/macOS/Linux identical, no missing binary errors, SSH-safe.
- **Lean binary** → default ~10 MB, `video` feature ~25 MB. No Electron, no Python runtime.
- **Async + cached** → UI never blocks, decode off main thread.
- **Batteries included** → each format has a pure Rust parser/decoder.

## 2. Core Dependencies — `Cargo.toml:1`

### 2.1 Runtime & UI (The Shell)

| Crate | Version | Role | Why Chosen |
|---|---|---|---|
| `ratatui` | 0.29 | TUI framework | Industry standard, double-buffered, 0 deps |
| `crossterm` | 0.28 | Terminal I/O | Cross-platform, Windows ConHost + VT support |
| `tokio` | 1.0 `full` | Async runtime | Worker pool, file watcher, channel |
| `directories` | 5.0 | XDG paths | Config `~/.config/tui-preview`, cache `~/.cache/tui-preview` |
| `serde` + `toml` | 1.0 / 0.8 | Config | TOML config parsing |
| `thiserror` | 2.0 | Error handling | Typed errors per handler |
| `clap` | 4.5 `derive` | CLI args | `tui-preview [PATH] [--theme dark]` |
| `tracing` + `tracing-subscriber` | 0.1 | Logging | File log `~/.cache/tui-preview/debug.log` |

### 2.2 Filesystem & Search

| Crate | Role |
|---|---|
| `walkdir` 2.5 | Recursive dir listing, sorted |
| `ignore` 0.4 | .gitignore respect |
| `mime_guess` 2.0 | Extension → MIME |
| `infer` 0.19 | Magic bytes detection (first 512B) |
| `nucleo-matcher` 0.3 | Fuzzy search (like fzf, pure Rust, fast) |
| `notify` 6.1 | File watcher (optional feature) |
| `sha2` 0.10 | Cache key hashing |

### 2.3 Image Preview (Pure Rust)

| Crate | Handles | Notes |
|---|---|---|
| `image` 0.25 `png,jpeg,gif,webp,bmp` | Raster images | Decode + resize via `imageops::Lanczos3` |
| `resvg` 0.43 + `usvg` | SVG | Rasterize SVG to RGBA, then same pipeline |
| `viuer` 0.9 **or custom** `term::graphics` | Kitty/Sixel/iTerm2 | Abstraction; we implement custom for control (see ARCHITECTURE.md) |
| `webp` 0.3 | WebP fallback | If `image` webp insufficient |

Performance: Resize to `area.width*2 × area.height*2` before terminal render → half-block doubles vertical resolution.

### 2.4 Text / CSV / Markdown (Pure Rust)

| Crate | Role |
|---|---|
| `syntect` 5.2 | Syntax highlighting (100+ langs), pure Rust, TextMate grammars |
| `bat` logic (reuse syntect) | Alternative ifNeed |
| `csv` 1.3 | CSV parse + `comfy-table` 7.1 for table widget |
| `pulldown-cmark` 0.12 | Markdown → styled Ratatui lines |
| `encoding_rs` 0.8 | Charset detection (UTF-8, Windows-1252) |
| `content_inspector` 0.2 | Binary vs text detection |

### 2.5 PDF (Pure Rust — Tradeoff Documented)

| Option | Purity | Quality | Choice |
|---|---|---|---|
| `lopdf` 0.35 + `pdf-extract` 0.8 | 100% pure | Text extraction excellent, raster none | **Use for text** |
| `mupdf` 0.4 | C binding | High-quality raster 150 DPI | Feature-gated |
| `pdfium-render` 0.8 | C++ binding (pdfium) | Best fidelity | Feature-gated alt |
| `hayro` 0.2 (experimental) | 100% pure Rust | Early, limited | Future |

**v1 Strategy:** Default = `lopdf` text + `mupdf` raster behind `pdf-raster` feature. If user demands 100% pure with no C, compile without feature → text-only PDF + metadata (page count via `lopdf`). Documented as known limitation.

### 2.6 Office Documents (Pure Rust)

| Crate | Format | Capability |
|---|---|---|
| `docx-rs` 0.4 | DOCX | Paragraphs, tables, headings → Ratatui text |
| `calamine` 0.26 | XLSX, XLS, ODS | Sheets → `comfy-table`, streaming, no full load |
| `pptx-rs` 0.1 / `dotrs` logic | PPTX | Slide titles + bullets → paginated text |
| `zip` 2.2 | All OOXML | Underlying zip (docx/pptx are zips) |

No `libreoffice` headless; pure Rust parse is faster (no 1s LO startup) but loses pixel-perfect layout — acceptable for preview.

### 2.7 Audio (Pure Rust — No mpv)

| Crate | Role |
|---|---|
| `symphonia` 0.5 `all` | Decode mp3, flac, wav, ogg, m4a pure Rust |
| `lofty` 0.22 | Tags: title, artist, album, duration, bitrate |
| `rodio` 0.20 | Playback `Sink`, pause/resume, pure Rust (ALSA/CoreAudio/WASAPI) |
| `hound` 3.0 | WAV helper |

Waveform: `symphonia` decode first 30s → downsample to 80 bars → Ratatui `Sparkline` widget.

### 2.8 Video (Pure Rust Compromise)

**Problem:** No production pure Rust H.264/H.265 decoder. Options:

| Approach | Purity | Thumbnail | Chosen |
|---|---|---|---|
| `ffmpeg-next` 7.1 | Rust binding → FFmpeg C libs | Full thumbnail + metadata | **Feature `video`** |
| `mp4` 0.14 + `matroska` | Pure Rust header parse | Metadata only (no frame) | **Default fallback** |

`Cargo.toml`:
```toml
ffmpeg-next = { version = "7", optional = true }
mp4 = { version = "0.14", optional = true }
[features]
default = []
video = ["ffmpeg-next"]
video-pure = ["mp4"] # metadata only
```
If `video` feature enabled, `cargo build` statically links FFmpeg via `ffmpeg-sys-next` — still `cargo` build, no runtime binary needed. Documented build req: `cargo xwin` or `vcpkg` on Windows.

### 2.9 Caching & Hashing

| Crate | Role |
|---|---|
| `lru` 0.12 | In-memory LRU 100 entries |
| `sha2` 0.10 | Key hash |
| `sled` OR `rusqlite` optional | Persistent cache index (v2); v1 uses filesystem + mtime |

### 2.10 Dev & Quality

| Crate/Tool | Role |
|---|---|
| `clippy` | `disallowed-methods = ["std::process::Command::new"]` enforces purity |
| `rustfmt` | Formatting |
| `cargo-deny` | License/ban audit |
| `cargo-audit` | CVE check |
| `criterion` | Benchmarks `benches/preview.rs` |
| `insta` | Snapshot tests for handlers |

## 3. Why Pure Rust Beats Shell-Out

| Criterion | Pure Rust (chosen) | Shell-out (rejected) |
|---|---|---|
| Windows support | Works, single binary | Requires `ffmpeg.exe` in PATH, fragile |
| SSH headless | No deps to install | Need 5 binaries on server |
| Startup | <80ms | +200ms per `Command::spawn` |
| Security | No shell injection, bounded decode | RCE via crafted filenames |
| Distribution | `cargo install` | Docker + apt-get parade |
| Error handling | Typed `Result` | Stringly `stderr` parse |

## 4. Binary Size & Performance Targets

| Profile | Size | Startup | Cached Preview | Cold Image | Cold PDF |
|---|---|---|---|---|---|
| `default` (no video) | ~10 MB (strip+LTO) | 60-80 ms | <30 ms | <300 ms | <800 ms |
| `--features video` | ~25 MB | 70-90 ms | <30 ms | <300 ms | + thumbnail 500 ms |

Optimization: `Cargo.toml:profile.release lto=true, codegen-units=1, strip=true, panic=abort`.

## 5. Feature Flags Matrix — `Cargo.toml:15`

```toml
[features]
default = []                # pure, no C libs, metadata-only video
pdf-raster = ["mupdf"]      # C dep, raster PDF pages
video = ["ffmpeg-next"]     # C dep, thumbnails
full = ["pdf-raster", "video"] # richest
```

Users choose trade-off. Docs explain each flag's build requirements.

## 6. Alternatives Rejected

- `tui-rs` (deprecated) → `ratatui`
- `termion` (Unix only) → `crossterm` (cross-platform)
- `ffmpeg` CLI → `ffmpeg-next` binding
- `libreoffice` CLI → `docx-rs`/`calamine`
- `poppler` CLI → `lopdf`/`mupdf`

## 7. Verification Commands

```powershell
cargo build --release              # pure default, no C libs
cargo build --release --features pdf-raster  # needs mupdf
cargo build --release --features video       # needs FFmpeg libs
cargo clippy -- -D warnings
cargo deny check
cargo test --all-features
```

This stack delivers **lightweight + fast + efficient + working + rich** — all Rust, no external runtime.

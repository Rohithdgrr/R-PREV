# TECH-STACK.md — Pure Rust Stack (Lightweight, Fast, Efficient)

> Constraint: **PURELY RUST**. No `std::process::Command` to `ffmpeg/soffice/pdftoppm` in core. Single binary via `cargo build --release`. All heavy lifting via crates.
> **Review fixes applied:** `panic="abort"` removed, `mupdf` (AGPL-3.0) replaced, `pptx-rs` (abandoned) replaced, `tokio full` trimmed, `notify` + `memmap2` + `quick-xml` added.

## 1. Stack Philosophy

- **Zero shell-out core** → Windows/macOS/Linux identical, no missing binary errors, SSH-safe.
- **Lean binary** → default ~10 MB, `full` ~25 MB. No Electron, no Python runtime.
- **Async + cached + quantized + timed** → UI never blocks, decode off main thread, single timeout enforcement.
- **Batteries included** → each format has a pure Rust parser/decoder.

## 2. Core Dependencies — `Cargo.toml:1`

### 2.1 Runtime & UI (The Shell)

| Crate | Version | Role | Why Chosen |
|---|---|---|---|
| `ratatui` | 0.29 | TUI framework | Industry standard, double-buffered, 0 deps |
| `crossterm` | 0.28 | Terminal I/O | Cross-platform, Windows ConHost + VT support |
| `tokio` | 1.0 `rt, rt-multi-thread, macros, time, sync, fs` | Async runtime | Trimmed from `full` — no net/signal/process (saves binary size + compile time) |
| `directories` | 5.0 | XDG paths | Config `~/.config/tui-preview`, cache `~/.cache/tui-preview` |
| `num_cpus` | 1.16 | Pool sizing | `(get()/2).clamp(2,6)` workers |
| `serde` + `toml` | 1.0 / 0.8 | Config | TOML config parsing |
| `thiserror` | 2.0 | Error handling | Typed errors per handler |
| `clap` | 4.5 `derive` | CLI args | `tui-preview [PATH] [--theme dark]` |
| `tracing` + `tracing-subscriber` | 0.1 | Logging | File log `~/.cache/tui-preview/debug.log` |

> Fix: `tokio full` → trimmed list. Never needed `net`, `signal`, `process`, `io-util` for a TUI previewer.

### 2.2 Filesystem & Search

| Crate | Role | Notes |
|---|---|---|
| `walkdir` 2.5 | Recursive dir listing, sorted |  |
| `ignore` 0.4 | .gitignore respect |  |
| `mime_guess` 2.0 | Extension → MIME |  |
| `infer` 0.19 | Magic bytes detection (first 512B) |  |
| `nucleo-matcher` 0.3 | Fuzzy search (like fzf, pure Rust, fast) |  |
| `notify` 6.1 **optional (`watch` feature)** | File watcher | Fix: was missing from Cargo.toml despite ARCHITECTURE.md; now `watch` feature gated |
| `sha2` 0.10 | Cache key hashing (quantized area) |  |
| `memmap2` 0.9 | Zero-copy large text via `Mmap` | Fix: added for SSH log-tailing |
| `gix` 0.66 **optional (`git` feature)** | Git-aware badges | New: modified/staged badges |

### 2.3 Image Preview (Pure Rust)

| Crate | Handles | Notes |
|---|---|---|
| `image` 0.25 `png,jpeg,gif,webp,bmp` | Raster images | Decode + resize via `imageops::Lanczos3` |
| `resvg` 0.43 + `usvg` | SVG | Rasterize SVG to RGBA, then same pipeline |
| `viuer` 0.9 **or custom** `term::graphics` | Kitty/Sixel/iTerm2 | Abstraction; we implement custom for control (see ARCHITECTURE.md) |
| `webp` 0.3 | WebP fallback | If `image` webp insufficient |

Performance: Resize to `quantized_area.width*2 × height*2` before terminal render → half-block doubles vertical resolution. Cache key quantized (8 cols × 4 rows) so resize-by-pixel doesn't churn.

### 2.4 Text / CSV / Markdown (Pure Rust)

| Crate | Role |
|---|---|
| `syntect` 5.2 | Syntax highlighting (100+ langs), pure Rust, TextMate grammars |
| `csv` 1.3 | CSV parse + `comfy-table` 7.1 for table widget |
| `pulldown-cmark` 0.12 | Markdown → styled Ratatui lines |
| `encoding_rs` 0.8 | Charset detection (UTF-8, Windows-1252) |
| `content_inspector` 0.2 | Binary vs text detection |
| `memmap2` 0.9 | Large file mmap (avoid read+copy) — NEW |

### 2.5 PDF (Pure Rust — Tradeoff Documented) — FIXED

| Option | Purity | License | Quality | Choice |
|---|---|---|---|---|
| `lopdf` 0.35 + `pdf-extract` 0.8 | 100% pure | MIT/Apache | Text extraction excellent, raster none | **Use for text (default)** |
| `pdfium-render` 0.8 | C++ binding (pdfium) | **Apache-2.0** | Best fidelity, 150 DPI | **Feature `pdf-raster` — CORRECT** |
| `mupdf` 0.4 | C binding | **AGPL-3.0** | High-quality | **REMOVED** — AGPL conflicts with MIT + cargo deny ban |
| `hayro` 0.2 (experimental) | 100% pure Rust | MIT | Early, limited | Future |

**Fix:** `mupdf` removed from `Cargo.toml` and all docs. It is AGPL-3.0 (MuPDF dual AGPL/commercial) — shipping a binary with `pdf-raster` would taint the whole binary with AGPL obligations, violating your MIT license and `cargo deny` policy that bans GPL/AGPL. Replaced with `pdfium-render` (Google PDFium, Apache-2.0, permissive). `pdfium-render` is still a C++ binding (not 100% pure Rust) — docs now honestly call `pdf-raster`/`full` "Rust + pdfium C++ lib", not "pure Rust".

**v1 Strategy:** Default = `lopdf` text + optional `pdfium-render` raster behind `pdf-raster` feature. Without feature → text-only PDF + metadata (page count via `lopdf`).

### 2.6 Office Documents (Pure Rust) — FIXED

| Crate | Format | Capability |
|---|---|---|
| `docx-rs` 0.4 | DOCX | Paragraphs, tables, headings → Ratatui text |
| `calamine` 0.26 | XLSX, XLS, ODS | Sheets → `comfy-table`, streaming, no full load |
| `zip` 2.2 + `quick-xml` 0.36 | **PPTX** | **NEW: in-house PPTX extractor** — slide titles/text runs via zip+quick-xml, replaces `pptx-rs` |
| `zip` 2.2 | All OOXML | Underlying zip (docx/pptx are zips) |

**Fix:** `pptx-rs 0.1` removed — one release, 1,800 downloads, no repo, abandoned. PPTX is a zip of XML files (`ppt/slides/slide1.xml`); a minimal extractor with `quick-xml` is more maintainable than an orphaned crate and you already need `zip` for docx/xlsx. Handler `src/preview/office.rs` + new `pptx` sub-module now uses `zip` + `quick-xml`.

No `libreoffice` headless; pure Rust parse is faster (no 1s LO startup) but loses pixel-perfect layout — acceptable for preview.

### 2.6b Archive Preview (NEW — cheap, high value)

| Crate | Format | Capability |
|---|---|---|
| `zip` 2.2 | ZIP | Entry list, sizes, compressed ratio |
| `tar` 0.4 + `flate2` 1.0 | TAR, TAR.GZ | Same |

New `src/preview/archive.rs` — entry table `Name | Size | Packed | Ratio` via existing `zip`; top Yazi/Ranger feature, close to free.

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
watch = ["notify"]
git = ["gix"]
pdf-raster = ["pdfium-render"] # was ["mupdf"] — AGPL
full = ["pdf-raster", "video", "watch", "git"]
```
If `video` feature enabled, `cargo build` statically links FFmpeg via `ffmpeg-sys-next` — still `cargo` build, no runtime binary needed. Documented build req: `cargo xwin` or `vcpkg` on Windows.

### 2.9 Caching & Hashing

| Crate | Role |
|---|---|
| `lru` 0.12 | In-memory LRU 100 entries |
| `sha2` 0.10 | Key hash (quantized area) |
| `num_cpus` 1.16 | Worker pool sizing |
| `sled` OR `rusqlite` optional | Persistent cache index (v2); v1 uses filesystem + mtime |

### 2.10 Dev & Quality

| Crate/Tool | Role |
|---|---|
| `clippy` | `disallowed-methods = ["std::process::Command::new"]` enforces purity |
| `rustfmt` | Formatting |
| `cargo-deny` | License/ban audit — now correctly bans AGPL (would have caught mupdf) |
| `cargo-audit` | CVE check |
| `criterion` | Benchmarks `benches/preview.rs` |
| `insta` | Snapshot tests for handlers |

## 3. Why Pure Rust Beats Shell-Out

| Criterion | Pure Rust (chosen) | Shell-out (rejected) |
|---|---|---|
| Windows support | Works, single binary | Requires `ffmpeg.exe` in PATH, fragile |
| SSH headless | No deps to install | Need 5 binaries on server |
| Startup | <80ms | +200ms per `Command::spawn` |
| Security | No shell injection, bounded decode, `catch_unwind` works (kept `panic=unwind`) | RCE via crafted filenames, abort crashes |
| Distribution | `cargo install` | Docker + apt-get parade |
| Error handling | Typed `Result` | Stringly `stderr` parse |

## 4. Binary Size & Performance Targets

| Profile | Size | Startup | Cached Preview | Cold Image | Cold PDF |
|---|---|---|---|---|---|
| `default` (no video) | ~10 MB (strip+LTO, trimmed tokio) | 60-80 ms | <30 ms | <300 ms | <800 ms |
| `--features full` | ~25 MB | 70-90 ms | <30 ms | <300 ms | + thumb 500 ms |

Optimization: `Cargo.toml:profile.release lto=true, codegen-units=1, strip=true` — **`panic="abort"` removed** so `catch_unwind` in handlers works; the size cost of keeping unwind is ~200KB, worth it for crash safety on untrusted files.

## 5. Feature Flags Matrix — `Cargo.toml:15`

```toml
[features]
default = []                          # pure, no C libs, metadata-only video, no watcher
pdf-raster = ["pdfium-render"]        # C++ pdfium (Apache-2.0), NOT mupdf AGPL
video = ["ffmpeg-next"]               # C dep, thumbnails
video-pure = ["mp4"]                  # pure header parse
watch = ["notify"]                    # fs watcher
git = ["gix"]                         # git badges
full = ["pdf-raster", "video", "watch", "git"] # richest
```

Users choose trade-off. Docs explain each flag's build requirements.

## 6. Alternatives Rejected

- `tui-rs` (deprecated) → `ratatui`
- `termion` (Unix only) → `crossterm` (cross-platform)
- `ffmpeg` CLI → `ffmpeg-next` binding (feature-gated)
- `libreoffice` CLI → `docx-rs`/`calamine`
- `poppler` CLI → `lopdf`/`pdfium-render` (was `mupdf`)
- `pptx-rs` (abandoned, 1 release) → `zip`+`quick-xml` in-house

## 7. Verification Commands

```powershell
cargo build --release              # pure default, no C libs
cargo build --release --features pdf-raster  # pdfium (Apache-2.0)
cargo build --release --features video       # needs FFmpeg libs
cargo clippy -- -D warnings
cargo deny check   # now bans AGPL — would fail if mupdf returned
cargo audit
cargo test --all-features
```

This stack delivers **lightweight + fast + efficient + working + rich** — Rust with honest licensing, no AGPL contamination, no abandoned deps.

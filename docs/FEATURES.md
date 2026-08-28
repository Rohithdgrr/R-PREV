# FEATURES.md — Rich Feature Catalog (Pure Rust, Lightweight, Fast)

> Every feature tagged by phase: ✅ MVP (Phase 1), 🔜 Phase 2, 💡 Future.

## 1. Core TUI Experience

| Feature | Description | Phase |
|---|---|---|
| Two-pane layout (30/70) | File list + preview, status bar | ✅ |
| Full keyboard nav `j/k, g/G, h/l` | Vim-style, no mouse needed | ✅ |
| Fuzzy search `/` | `nucleo-matcher` live filter, Esc to clear | ✅ |
| Fullscreen preview `f` | Toggle handler output to full terminal | ✅ |
| Help overlay `?` | Keybinds cheatsheet, handler info | ✅ |
| Theme dark/light | `config.toml` + auto truecolor detect | ✅ |
| Hidden toggle `h` | Show dotfiles | ✅ |
| Parent nav `Backspace` | Up one dir | ✅ |
| Yank path `y` | Copy selected path to clipboard (via `arboard` pure Rust) | 🔜 |
| File ops `d, r, n` | Delete/rename/new file (confirm modal) | 💡 |
| Sort toggle `s` | Name/size/mtime, dirs-first toggle | 🔜 |
| Bookmarks `m` + `'` | Mark dir, jump back | 💡 |

## 2. Image Preview — Easiest & Most Polished

| Format | Handler | Features |
|---|---|---|
| PNG, JPEG, GIF, WEBP, BMP | `image` crate | ✅ Resize Lanczos3, metadata (dim, size), EXIF panel via `kamadak-exif`/`little_exif` (🔜) |
| SVG | `resvg` + `usvg` | ✅ Raster to RGBA, scale to pane |
| ICO, TIFF | `image` | 🔜 |
| AVIF | `image` + `libavif` binding | 💡 (behind feature) |

**Rendering:**
- ✅ Kitty Graphics Protocol (WezTerm, Kitty, Ghostty)
- ✅ Sixel (Foot, Windows Terminal 1.22+)
- ✅ iTerm2 Inline (macOS)
- ✅ Fallback half-block `▀` truecolor — 2 pixels per cell, always works
- 🔜 Zoom `+/−` and pan `hjkl` in fullscreen
- 🔜 Slideshow `a` auto-advance 2s

**Metadata bar:** `4032×3024 • 4.2MB • PNG • 3 days ago` via `src/preview/meta.rs:1`

## 3. Text & Code

| Feature | Details | Phase |
|---|---|---|
| Syntax highlight | `syntect` 100+ languages, TextMate themes `base16` | ✅ |
| Binary guard | `content_inspector` detects binary → show hex/metadata not garbage | ✅ |
| Encoding | `encoding_rs` auto UTF-8/Windows-1252 | ✅ |
| Large file | Limit 2MB/5000 lines, truncate + "… +N lines" | ✅ |
| Line numbers `L` | Toggle gutter | 🔜 |
| Search in file `Ctrl-F` | Highlight matches | 💡 |
| Markdown render | `pulldown-cmark` → styled headings/lists/code | ✅ |
| CSV/TSV table | `csv` sniff delimiter → `comfy-table` → Ratatui Table, 100 rows + header | ✅ |
| JSON pretty | `serde_json` formatted + syntect json | 🔜 |
| Log tail `T` | Follow file like `tail -f` via `notify` | 💡 |

## 4. PDF

| Feature | Details | Phase |
|---|---|---|
| Text extraction | `lopdf` + `pdf-extract` — first 2 pages, searchable | ✅ |
| Page count + author | Via `lopdf::Document::trailer` | ✅ |
| Raster first page (opt) | `pdfium-render` (Apache-2.0) at 150 DPI → image pane split | 🔜 (`--features pdf-raster`, NOT `mupdf` AGPL) |
| Page nav `n/p` | Next/prev page in fullscreen, cache per page | 🔜 |
| In-doc search `Ctrl-F` | Search within extracted text, highlight matches | 🔜 (cheap, high value) |
| Thumbnail strip | Vertical filmstrip of pages (future) | 💡 |

*Pure Rust default = text-only; raster is opt-in `pdfium-render` (Apache-2.0) via `pdf-raster` feature; `mupdf` AGPL removed.*

## 5. Office Documents — No LibreOffice Needed

| Format | Crate | Rendering |
|---|---|---|
| DOCX | `docx-rs` | ✅ Paragraphs, headings, tables → styled text |
| XLSX, XLS, ODS | `calamine` | ✅ First sheet as Table, `Tab`/`Shift-Tab` switch sheets, column widths auto |
| PPTX | `zip` + `quick-xml` (in-house, `pptx-rs` removed — abandoned) | ✅ Slide titles + bullets paginated, `n/p` per slide |
| DOC (old) | `calamine` + `encoding_rs` | 🔜 Text extraction only |

**Features:**
- ✅ Table view for XLSX with frozen header
- ✅ Formula display (show value, `f` toggle formula)
- 🔜 Embedded image extraction (docx media/ folder → image preview)
- 💡 Slide thumbnails (render slide XML via `resvg` — heavy)

## 6. Audio — Pure Rust Playback

| Feature | Details | Phase |
|---|---|---|
| Formats | MP3, FLAC, WAV, OGG, M4A/AAC via `symphonia` | ✅ |
| Metadata | Title/artist/album/bitrate/sample-rate/duration via `lofty` | ✅ |
| Waveform | 30s decode → 80-bar Sparkline | ✅ |
| Playback `Space` | `rodio` Sink play/pause `Space`, stop `s`, auto-stop on nav | ✅ |
| Volume `-/=` | Sink volume 0.0-1.0 | 🔜 |
| Seek `</>` | 5s forward/back (decode seek) | 💡 |
| Playlist `a` | Queue all audio in dir | 💡 |

## 7. Video — Feature-Gated

| Feature | Details | Phase |
|---|---|---|
| Metadata | Duration, resolution, fps, codec, bitrate via `ffmpeg-next` or `mp4` header | ✅ (metadata) / 🔜 (thumbnail) |
| Thumbnail | Frame at 10% duration -> image pipeline | 🔜 (`--features video`) |
| Strip preview `t` | 5 thumbnails across timeline | 💡 |
| Launch external `o` | Open with `$VIDEO_PLAYER` (mpv/vlc) | ✅ (spawns via `open` crate, only external spawn allowed) |
| Pure Rust fallback | `mp4` crate header parse without FFmpeg, no thumbnail | ✅ default |

## 8. Filesystem & Navigation

| Feature | Details |
|---|---|
| Directory preview | Summary: `42 entries (30 files, 12 dirs) • 1.2GB` + largest files | ✅ |
| Dir size on demand `D` | `du`-style async walk + sum, cached (NEW — `src/fs/du.rs:1`) | 🔜 |
| Sorting | Dirs first + alpha; `s` cycles size/mtime | 🔜 |
| .gitignore respect | `ignore` crate respects `.gitignore` + `.tui-ignore` | ✅ |
| Symlink | Show `→ target`, depth limit 10, broken highlight red | ✅ |
| Permissions | `rwxr-xr-x` + size humanized `4.2M` in list | ✅ |
| Git-aware badges | `gix` crate: `M` modified, `?` untracked, `S` staged (NEW, `--features git`) | 🔜 |
| Watch (`notify`) | Live reload on fs change — feature-gated `watch` (was missing from Cargo.toml, now fixed) | 🔜 |
| Archive preview `zip/tar/tar.gz` | Entry listing `Name | Size | Packed | Ratio` via `zip`/`tar`+`flate2` (NEW, cheap — Yazi top feature) | 🔜 |

## 9. Caching & Performance

| Feature | Details | Phase |
|---|---|---|
| Two-tier cache | Mem LRU 100 + Disk 500MB `~/.cache/tui-preview` | ✅ |
| Key = hash(path+mtime+size+**quantized_area**) | Quantized 8 cols × 4 rows — no churn on pixel resize (FIXED) | ✅ |
| Async + cancellation + 5s timeout | Router `tokio::time::timeout` centralized; fast scroll aborts stale | ✅ |
| Sized blocking pool | `(num_cpus/2).clamp(2,6)` via `num_cpus`, configurable `worker_threads` (FIXED: was hardcoded 2) | ✅ |
| Large text `memmap2` | Zero-copy mmap for files >1MB (NEW) | ✅ |
| Size guards | 50MB image / 100MB pdf threshold → "Enter to force" | ✅ |
| Benchmarks | `cargo bench` asserts <300ms cold image, regression gate in CI | 🔜 |
| Clear cache | `tui-preview --clear-cache` | ✅ |
| Keep `panic=unwind` | `catch_unwind` works for malformed files (FIXED: was `abort`) | ✅ |

## 10. Configuration & Customization

| Feature | Details |
|---|---|
| `config.toml` | `~/.config/tui-preview/config.toml` — theme, keys, cache, limits | ✅ |
| Key remap | Any Action → key string in TOML | ✅ |
| Theme | `dark`/`light`, syntect theme `base16-ocean.dark` etc. | ✅ |
| Env overrides | `TUI_PREVIEW_THEME=light` | 🔜 |
| `--init-config` | Generate default config file | ✅ |

## 11. Integration & Scriptability

| Feature | Details |
|---|---|
| CLI `tui-preview [PATH]` | Open dir or file, `tui-preview ./report.pdf` focuses it | ✅ |
| Pipe `ls \| tui-preview` | Accept file list on stdin (future) | 💡 |
| `fzf` integration | `fzf --preview 'tui-preview --preview {}'` headless preview mode `--preview <file>` prints text/renders image escape | 🔜 |
| `yazi`/`ranger` previewer | Implement `--preview` subcommand for file managers | 🔜 |
| `--export-thumbs <dir>` | Batch export thumbnails without TUI (automation/CI) | 🔜 |
| `--bench` | Benchmark mode for CI artifacts | 🔜 |
| `$EDITOR` jump `e` | Open text file in `$EDITOR` (nvim/vim) alongside `o` for OS open (NEW — terminal user top request) | 🔜 |
| `EXIF` panel `x` | Image EXIF/metadata overlay via `kamadak-exif` (NEW, cheap) | 🔜 |
| Headless server | Works over SSH (no GPU, Sixel degrades gracefully) | ✅ |

## 12. Accessibility & Polish

| Feature | Details |
|---|---|
| Truecolor auto | Detect `COLORTERM=truecolor`, fallback 256 | ✅ |
| Unicode correct | `unicode-width` for table alignment | ✅ |
| Error never crash | Every handler `Result` → red Error pane + fallback | ✅ |
| Logging | `~/.cache/tui-preview/debug.log` with `RUST_LOG=debug` | ✅ |
| Help `?` | Fullscreen overlay with all binds + handler caps | ✅ |
| Version `V` | Show build features + term caps | ✅ |

## 13. Feature Flags Summary

| Flag | Adds | Binary | License |
|---|---|---|---|
| `default` | Pure Rust, archive+text/image/pdf-text/office/audio/meta, `memmap2` | ~10 MB | MIT |
| `pdf-raster` | `pdfium-render` page raster (Apache-2.0, NOT mupdf AGPL) | +5 MB | Apache-2.0 |
| `video` | `ffmpeg-next` thumb + metadata | +15 MB | Varies (FFmpeg) |
| `watch` | `notify` file watcher | +0.5 MB | — |
| `git` | `gix` git badges | +1 MB | — |
| `full` | All above | ~25 MB | — |

Install choose:

```powershell
cargo install tui-preview                         # lightweight pure
cargo install tui-preview --features pdf-raster    # + pdf images
cargo install tui-preview --features video         # + video thumbs
cargo install tui-preview --features full          # richest
```

## 14. Non-Features (Explicitly Out of Scope v1)

- Video smooth playback (use `o` to launch mpv)
- Document editing (preview only)
- PDF annotation/zoom beyond 150 DPI
- Office pixel-perfect layout
- DRM/encrypted docs (show "encrypted" error)

All decisions keep **lightweight + fast + working** — rich preview, not editor.

## 15. Feature Dependency Graph

```
FileList + Router (must, with centralized 5s timeout + quantized cache)
  ├─ ImageHandler (must, easiest win) + EXIF panel
  ├─ Text/Csv/Md (must, memmap2 for large)
  ├─ ArchiveHandler (must, cheap via zip/tar)
  ├─ Pdf text (must) ── Pdf raster via pdfium-render (opt, Apache-2.0)
  ├─ Office (must text, zip+quick-xml pptx) (no pptx-rs)
  ├─ Audio meta+play (must)
  └─ Video meta (must) ── Video thumb via ffmpeg-next (opt)
Cache (must, sized pool + quantized keys)
Term Graphics (must fallback half-block)
Search (must) + Git badges (opt) + Dir du (opt) + $EDITOR jump
```

Build in that order — see PHASEWISEPLAN.md.

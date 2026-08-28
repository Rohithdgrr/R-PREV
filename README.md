# tui-preview — Pure Rust Terminal File Previewer

> **Lightweight • Fast • Efficient • Working • Rich** — keyboard-driven universal previewer for developers/power users. Pure Rust, single binary, no `ffmpeg`/`soffice` shell-out in core. See `docs/` for professional-grade design.

```
┌──────────────────────┬────────────────────────────────────────┐
│ > project/           │ Preview: photo.png                     │
│   photo.png ◄──────  │  [image via Kitty/Sixel or half-block]│
│   report.pdf         │  4032×3024 PNG • 4.2MB                 │
│   sales.xlsx         │                                        │
│   song.mp3           │  j/k nav  / search  f fullscreen ? help│
└──────────────────────┴────────────────────────────────────────┘
```

## Quick Start (Pure Rust, no deps)

```powershell
cargo install --path .        # ~10MB, pure Rust
tui-preview .                 # open current dir
tui-preview ./report.pdf      # focus file
```

## Why tui-preview?

- **Fast triage** over SSH where GUI apps fail.
- **Keyboard** faster than double-click for power users.
- **Cached + async** — cached preview <30ms, UI never blocks.
- **Pure Rust** — no external binaries, Windows/Linux/macOS identical.

> For full video playback or perfect Office layout, press `o` to open in native app — preview, not replace.

## Rich Feature Tour

| Preview | Pure Rust Crate | Notes |
|---|---|---|
| Images png/jpg/svg/webp | `image`, `resvg` | Kitty/Sixel/iTerm2 + half-block fallback |
| Text/Code | `syntect` | 100+ languages, 2MB limit |
| CSV/TSV | `csv`, `comfy-table` | Table, delimiter auto |
| Markdown | `pulldown-cmark` | Styled headings |
| PDF | `lopdf`, `pdf-extract` (+ `pdfium-render` Apache-2.0 feature, was `mupdf` AGPL removed) | Text + optional 150 DPI image |
| DOCX/XLSX/PPTX | `docx-rs`, `calamine`, `zip`+`quick-xml` pptx (was `pptx-rs` abandoned) | Tables, sheets via Tab, slides n/p |
| ZIP/TAR/TGZ | `zip`, `tar`+`flate2` | Archive entry listing (NEW) |
| Audio mp3/flac/wav | `symphonia`, `lofty`, `rodio` | Meta + waveform + Space play |
| Video mp4/mkv | `mp4` (meta) / `ffmpeg-next` feature (thumb) | Thumbnail at 10% |

Audio playback pure Rust (`rodio`), no `mpv`.

## Install Matrix

```powershell
cargo install --path .                         # pure (recommended)
cargo install --path . --features pdf-raster   # + pdf images
cargo install --path . --features video        # + video thumbs (needs FFmpeg)
cargo install --path . --features full         # richest ~25MB
```

## Keybinds

`j/k` nav, `/` search, `Enter` enter/dir or fullscreen file, `Backspace` parent, `h` hidden, `f` fullscreen, `n/p` pdf/pptx pages, `Tab` xlsx sheets, `Space` audio play, `o` open external, `?` help, `q` quit.

Full guide: `docs/USAGE.md`.

## Docs

| Doc | Read |
|---|---|
| `docs/ARCHITECTURE.md` | System layers, modules, data flow |
| `docs/TECH-STACK.md` | Pure Rust crates, feature flags, binary sizes |
| `docs/BACKEND.md` | Preview router + 7 handlers + cache |
| `docs/WORKFLOW.md` | Dev loop, runtime journeys |
| `docs/WORKING.md` | End-to-end how it works |
| `docs/FEATURES.md` | Rich catalog by phase |
| `docs/PHASEWISEPLAN.md` | 6 phases, 4 weeks MVP→v1 |
| `docs/TODO.md` | Checkable tasks file:line |
| `docs/USAGE.md` | User guide + troubleshooting |
| `docs/CONFIG.md` | Config TOML reference |
| `docs/PERFORMANCE.md` | Budgets, bench, cache tuning |
| `docs/SECURITY.md` | Guards, limits, sandbox |

## Project Structure

```
src/
  main.rs, app.rs, event.rs, config.rs, error.rs
  fs/, preview/, term/, cache/, ui/
docs/  (this design suite)
fixtures/  (sample files for manual test)
```

## Development

```powershell
cargo run -- .              # dev run
cargo watch -x "run -- ."   # auto-reload
cargo clippy -- -D warnings
cargo test
cargo bench
```

See `docs/WORKFLOW.md` for full workflow.

## Performance Targets

Startup <80ms, cached <30ms, cold image <300ms, disk cache 500MB LRU, mem 100 entries.

See `docs/PERFORMANCE.md`.

## Security

No `Command::new` in core, size limits, decompression bomb guards. See `docs/SECURITY.md`.

## License

MIT (or Apache-2.0) — choose during `cargo init`.

## Roadmap

- `0.1.0` Phase2 MVP+cache
- `0.2.0` Phase3 PDF/Office
- `0.5.0` Phase4 video
- `1.0.0` Phase5 stable release

Full plan: `docs/PHASEWISEPLAN.md`.

---

*Build: `cargo build --release` — pure Rust, single binary. For richest, `cargo build --release --features full`.*


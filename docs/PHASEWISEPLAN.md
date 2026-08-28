# PHASEWISEPLAN.md — Phased Execution Plan (Pure Rust, Lightweight → Rich)

> Timeline: 4 weeks MVP → rich v1. Each phase ends with `cargo build --release` + manual test on fixtures. Phases are sequential; each phase's Definition of Done is measurable.

## Phase 0 — Scaffolding (Days 1-2) — Foundation

**Goal:** Window opens, navigates dirs, empty preview. No handlers yet.

| Task | Files | Done When |
|---|---|---|
| `cargo init --name tui-preview` + Cargo.toml (default feature set) | `Cargo.toml:1`, `src/main.rs:1` | `cargo build` passes |
| App state + event loop + Ratatui double-pane | `src/app.rs:1`, `src/event.rs:1`, `src/ui/layout.rs:1` | `cargo run -- .` shows file list, `j/k` moves |
| `fs::list_dir` sorted dirs-first, hidden filter | `src/fs/mod.rs:1` | Correct order, `h` toggles dotfiles |
| Config load defaults | `src/config.rs:1` | `~/.config/tui-preview/config.toml` created |
| Clippy + rustfmt + `disallowed-methods` | `clippy.toml`, `.rustfmt.toml` | `cargo clippy` clean |
| Fixtures dir | `fixtures/{sample.png, notes.md, demo.pdf…}` | Exists for later tests |

**Exit criteria:** Binary ~2MB, startup <50ms, no preview decode yet, all tests `cargo test` pass.

## Phase 1 — Core Preview MVP (Days 3-8) — Highest Value, Lowest Risk

**Goal:** 80% of daily use: text, images, csv, directories. Pure Rust only.

| Task | Crate | Est. |
|---|---|---|
| TextHandler: syntect + encoding_rs + binary guard | `syntect` | 1 day |
| CsvHandler: csv + comfy-table | `csv`, `comfy-table` | 0.5 day |
| DirectoryHandler: summary (counts, largest) | std | 0.5 day |
| ImageHandler: image + resvg + half-block fallback | `image`, `resvg` | 1.5 days |
| Term::graphics fallback half-block | `image::imageops` | 0.5 day |
| Markdown via pulldown-cmark | `pulldown-cmark` | 0.5 day |
| Status bar: size, mtime, mime, handler name | `src/ui/status.rs:1` | 0.5 day |
| Error pane (red) + fallback | `src/ui/preview_pane.rs:1` | 0.5 day |

**Files touched:** `src/preview/text.rs:1`, `csv.rs:1`, `image.rs:1`, `src/term/graphics.rs:1`, `src/ui/*`

**Exit criteria demo:**
```powershell
cargo run -- ./fixtures
# photo.png renders as half-block (or Kitty if supported)
# notes.md rendered with headings bold
# sales.csv shows 100-row table
# main.rs syntax highlighted
# binary.exe shows "binary file • 2.1MB"
```

## Phase 2 — Cache & Performance + Audio + Search (Days 9-14)

**Goal:** Feels fast; can Play audio; can find files.

| Task | Detail | Est. |
|---|---|---|
| Cache two-tier (mem LRU 100 + disk 500MB) | `src/cache/mod.rs:1`, `key.rs:1` | 1 day |
| Async worker pool 2 + cancellation | `src/app.rs:80` JoinSet + AbortHandle | 1 day |
| Size guards (50MB image, etc.) + "Enter to force" | `src/preview/mod.rs:30` | 0.5 day |
| AudioHandler: lofty meta + symphonia waveform + rodio play | `symphonia`, `lofty`, `rodio` | 1.5 days |
| Fuzzy search `/` via nucleo-matcher | `src/ui/search.rs:1` | 1 day |
| Help overlay `?` + fullscreen `f` | `src/ui/layout.rs:1` | 0.5 day |
| Tracing to file + `RUST_LOG` | `src/main.rs:30` | 0.5 day |

**Perf targets after Phase 2:**
- Startup <80ms, cached <30ms, cold image <300ms, fast scroll no jank.
- Bench: `cargo bench --bench preview` passes budgets.

## Phase 3 — PDF + Office (Days 15-20) — Heavy Documents

**Goal:** Reports, invoices, spreadsheets preview without LO.

| Task | Crate | Est. |
|---|---|---|
| PdfHandler text: lopdf + pdf-extract (first 2 pages) | `lopdf`, `pdf-extract` | 1 day |
| PdfHandler raster (feature-gated): mupdf pixmap 150dpi | `mupdf` (`pdf-raster` feature) | 1 day |
| OfficeHandler docx: docx-rs paragraphs/tables | `docx-rs` | 1 day |
| OfficeHandler xlsx: calamine first sheet + Tab switch | `calamine` | 1 day |
| OfficeHandler pptx: pptx-rs slides + n/p paginated | `pptx-rs` | 0.5 day |
| Pagination keys `n/p` for pdf/pptx | `src/app.rs:120` Mode::FullscreenPreview | 0.5 day |

**Exit criteria:**
```powershell
cargo run -- ./fixtures/report.pdf      # text pane shows
cargo run --features pdf-raster -- ./fixtures/report.pdf # split text+image
cargo run -- ./fixtures/sales.xlsx      # Table with Tab sheet switch
cargo run -- ./fixtures/deck.pptx       # Slide 1/5 + n/p
```

## Phase 4 — Video (Opt-in) + Terminal Graphics Polish (Days 21-24)

**Goal:** Video metadata + thumbnails; Kitty/Sixel polish.

| Task | Detail | Est. |
|---|---|---|
| VideoMetaHandler default (mp4 header) pure Rust | `mp4` crate | 0.5 day |
| VideoHandler with ffmpeg-next thumbnail @10% | `ffmpeg-next` (`video` feature) | 1 day |
| Term caps detection: Kitty/Sixel/iTerm2 query + viuer fallback | `src/term/capabilities.rs:1` | 1 day |
| Video thumb pipeline: frame → RgbaImage → cache → graphics | `src/preview/video.rs:1` | 0.5 day |
| External open `o` via `open` crate ($VIDEO_PLAYER) | `open` | 0.5 day |
| Resize handling: on terminal resize, invalidate cache, re-render | `src/app.rs:60` | 0.5 day |

**Build matrix:**
```powershell
cargo build --release                         # metadata-only video
cargo build --release --features video         # thumbnails
cargo build --release --features full          # video + pdf raster
```

## Phase 5 — Polish, Config, Distribution (Days 25-28)

**Goal:** Production-grade release.

| Task | Detail | Est. |
|---|---|---|
| Config keys remap + theme + --init-config | `src/config.rs:1` | 0.5 day |
| --clear-cache, --version, --help | `src/main.rs:1` via clap | 0.5 day |
| --preview <file> headless for fzf | `src/main.rs:40` print preview + exit | 0.5 day |
| Release profile: LTO, strip, codegen-units=1 | `Cargo.toml:profile.release` | 0.5 day |
| Cross-build scripts + GitHub Release | `cargo-dist`, `.github/workflows/ci.yml` | 1 day |
| Benchmark suite criterion + RSS check | `benches/preview.rs` | 0.5 day |
| README + docs polish + fixtures golden tests | `tests/golden/*` via `insta` | 1 day |

## Phase 6 — Future (Post-v1) — Rich Add-ons

| Feature | Effort | Trigger |
|---|---|---|
| Zoom/pan for images in fullscreen | M | User request |
| Slideshow `a` 2s per image | S | Easy win |
| Sheet formula toggle `f` in XLSX | S | Office polish |
| Playlist queue `a` for audio dir | M | Audio power users |
| 5-thumb video strip `t` | M | Video polish |
| File ops delete/rename + notify watcher | M | If requested |
| Lua/Rhai handler plugins | L | Ecosystem |
| `--export-thumbs` batch CI mode | S | Automation users |
| Yank path `y` via arboard | S | Clipboard |

## Gantt View

```
Week1: [Phase0 Scaffolding][==== Phase1 MVP text/image/csv ====]
Week2: [==== Phase2 Cache+Audio+Search =======================]
Week3: [==== Phase3 PDF+Office ================================]
Week4: [== Phase4 Video/Graphics ==][== Phase5 Polish/Release ==]
Post:  [ Phase6 future add-ons as needed                 ]
```

## Per-Phase Deliverables & Gates

| Phase | Artifact | Test Command | Gate |
|---|---|---|---|
| 0 | Binary with file list | `cargo run -- .` | Manual nav works |
| 1 | Text/image/csv preview | `cargo test --lib` + manual fixtures | All fixtures render |
| 2 | Cache+audio+search | `cargo bench` <300ms | Perf budgets pass |
| 3 | PDF/Office | `cargo run --features pdf-raster -- ./fixtures` | Report+sheet+titles correct |
| 4 | Video/Graphics | `cargo build --features video && cargo test` | Thumbnail appears in Kitty term |
| 5 | Release | `cargo build --release --features full` | 3-arch artifacts, README done |

## Risk Mitigation by Phase

| Risk | Phase It Hits | Mitigation |
|---|---|---|
| Unknown mupdf build on Windows | 3 | Gate behind feature, fallback text-only, CI windows-latest |
| ffmpeg-next static link fails | 4 | Feature-gated, default is pure metadata via `mp4`, docs show vcpkg |
| Sixel not in ConHost | 4 | Fallback half-block always works, not a blocker |
| calamine large XLSX slow | 3 | Streaming, limit 100 rows + "… +N rows", Tab pagination |

## Definition of Done (Every Phase)

- [ ] `cargo build --release` + `cargo build --release --features full` both pass
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo test` pass (unit + `insta` golden)
- [ ] Manual fixture run documented in PR description
- [ ] Docs updated (ARCHITECTURE/WORKING if new handler)
- [ ] Perf budget not regressed (`cargo bench` if exists)

## Milestone Versioning

- `0.1.0` after Phase 2 (MVP + cache) — tag `mvp`
- `0.2.0` after Phase 3 (+ PDF/Office) — tag `docs`
- `0.5.0` after Phase 4 (+ video) — tag `media`
- `1.0.0` after Phase 5 polish — first stable release

This plan keeps **lightweight + fast** early, adds **rich** later, never breaks pure Rust core.


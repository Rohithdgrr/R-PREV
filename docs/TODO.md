# TODO.md — Actionable Checklist (Pure Rust, Rich Design)

> Check off in order. Each item references file:line to create. Use `cargo test` after each section.

## Phase 0 — Scaffolding (2 days)

- [ ] `Cargo.toml:1` — create with `default = []`, `pdf-raster`, `video`, `full` features (see TECH-STACK.md)
- [ ] `src/main.rs:1` — `#[tokio::main]`, clap CLI (`[PATH]`, `--clear-cache`, `--init-config`, `--version`), tracing, raw mode enter/leave
- [ ] `src/app.rs:1` — `App` struct, `Mode` enum, `Action` reducer, dirty flag
- [ ] `src/event.rs:1` — `EventStream` + key map `j/k/q/?//f/h/Enter/Esc/Space`
- [ ] `src/config.rs:1` — `Config` TOML load from `ProjectDirs`, defaults if missing
- [ ] `src/fs/mod.rs:1` — `Entry`, `list_dir` sorted dirs-first, hidden filter, symlink depth 10
- [ ] `src/ui/layout.rs:1` — Ratatui `Layout::horizontal([30,70]) + footer 1`, `FileList` List widget
- [ ] `src/ui/status.rs:1` — status bar with mode, selected index, hint
- [ ] `src/error.rs:1` — `thiserror` `PreviewError` enum
- [ ] `clippy.toml` — `disallowed-methods = ["std::process::Command::new"]`
- [ ] `fixtures/` — add `sample.png`, `notes.md`, `sample.csv` for dev

**Verify:** `cargo clippy && cargo run -- .` navigates, no preview decode.

## Phase 1 — Core Preview MVP (5 days)

- [ ] `src/preview/mod.rs:1` — `PreviewHandler` trait + `Router` + `FileMeta` + `PreviewResult` enums
- [ ] `src/preview/meta.rs:1` — `file_meta(path)` size, mtime, mime, magic
- [ ] `src/preview/text.rs:1` — binary guard `content_inspector`, `encoding_rs`, `syntect` highlight, 2MB/5000 line limit
- [ ] `src/preview/csv.rs:1` — delimiter sniff `,;|\t`, `csv::Reader`, `TableData {headers, rows: 100}` 
- [ ] `src/preview/image.rs:1` — `image::open` + `resvg` SVG, resize Lanczos3 to `area*2`, size guard 50MB
- [ ] `src/term/capabilities.rs:1` — `detect()` Kitty/Sixel/iTerm2/truecolor
- [ ] `src/term/graphics.rs:1` — `render_image` with fallback half-block `▀` (always works)
- [ ] `src/ui/preview_pane.rs:1` — match `PreviewResult` → `Paragraph`/`Table`/`Image` via `term::graphics`
- [ ] `src/cache/mod.rs:1` scaffold (no-op cache for now, just struct)
- [ ] Tests: `tests/golden/text.rs` insta snapshots for `.rs` highlight + markdown

**Verify:** `cargo run -- ./fixtures` — each fixture type renders.

## Phase 2 — Cache + Perf + Audio + Search (5 days)

- [ ] `src/cache/key.rs:1` — `key(path, area, version) = sha256(canonical+mtime+size+area+ver)`
- [ ] `src/cache/mod.rs:1` — mem `LruCache` 100 + disk `~/.cache/tui-preview/thumbs/<hash>.png`, 500MB evict, `get`/`put` async
- [ ] `src/app.rs:80` — `JoinSet` worker pool 2 + `AbortHandle` cancellation on selection change, `mpsc` channel 8
- [ ] Size guards in `router.rs:30` — TooLarge → `Error("Press Enter to force")` + force flag
- [ ] `src/preview/audio.rs:1` — `lofty` tags + `symphonia` 30s waveform downsample 80 bars + `rodio` Sink, `Space` play/pause `s` stop
- [ ] `src/ui/search.rs:1` — `SearchBar` widget, `nucleo-matcher` filter live, `App.files_filtered`
- [ ] Help overlay `?` + fullscreen toggle `f` in `src/ui/layout.rs:1`
- [ ] `src/main.rs:30` — `tracing_subscriber::fmt` to `~/.cache/tui-preview/debug.log`, `RUST_LOG=debug`
- [ ] Bench: `benches/preview.rs` criterion for image/csv/text cold vs cached
- [ ] Perf budgets: assert <300ms image, <30ms cached

**Verify:** `cargo bench` passes budgets, fast scroll no jank, audio plays.

## Phase 3 — PDF + Office (5 days)

- [ ] `src/preview/pdf.rs:1` — `lopdf` page count + `pdf-extract` text (2 pages), return `Text` + `meta`
- [ ] `src/preview/pdf.rs:40` feature-gated `mupdf::Document::open` pixmap 150dpi → `Image` split pane, cache per page
- [ ] `Cargo.toml` feature `pdf-raster = ["mupdf"]` documented
- [ ] `src/preview/office.rs:1` — `docx-rs` read_docx → paragraphs/tables → `Text`
- [ ] `src/preview/office.rs:40` — `calamine` open_workbook_auto → `worksheet_range_at(0)` → `Table`, sheet picker `Tab`
- [ ] `src/preview/office.rs:70` — `pptx-rs` slides → paginated `Text`, `n/p` next/prev slide
- [ ] `src/app.rs:120` — `Mode::FullscreenPreview` pagination `n/p` delegates to handler if pdf/pptx
- [ ] Fixtures: `fixtures/report.pdf`, `fixtures/sales.xlsx`, `fixtures/deck.pptx` + golden tests

**Verify:** `cargo run --features pdf-raster -- ./fixtures/report.pdf` split view; xlsx Tab switches sheets.

## Phase 4 — Video + Term Polish (3 days)

- [ ] `src/preview/video.rs:1` — default `mp4::Mp4Reader` header metadata (duration/res/codec) → `Text` + hint
- [ ] `src/preview/video.rs:40` `#[cfg(feature="video")]` ffmpeg-next: `format::input` → metadata + seek 10% → frame → `RgbaImage` → `Image`, cache thumb
- [ ] `Cargo.toml` feature `video = ["ffmpeg-next"]` + build docs (vcpkg/LLVM on Windows)
- [ ] `src/term/capabilities.rs:1` — probe `CSI ? 1;2 S`, env `KITTY_WINDOW_ID`, `TERM_PROGRAM`
- [ ] `src/term/graphics.rs:40` — Kitty chunk 4096, Sixel `ESC P q`, iTerm2 `ESC ]1337`, fallback verified in Windows Terminal + WezTerm
- [ ] External open `o` via `open` crate → `open::that(path)` (only allowed Command spawn, documented)
- [ ] Resize: `Event::Resize(w,h)` → invalidate cache key (area changed) → re-render current preview

**Verify:** `cargo build --features video` thumb appears in Kitty term; `cargo build` without feature shows metadata + hint.

## Phase 5 — Polish & Release (3 days)

- [ ] `src/config.rs:1` — key remap `keys` table, theme `dark/light` syntect ThemeSet, `preview_delay_ms` debounce
- [ ] `src/main.rs:1` clap adds `--clear-cache` (delete `~/.cache/tui-preview`), `--init-config`, `--preview <file>` headless
- [ ] Headless `--preview <file>`: call router + print text or base64 image escape, exit 0 (for fzf)
- [ ] `Cargo.toml:profile.release` — `lto=true, codegen-units=1, strip=true, panic=abort`
- [ ] `benches/preview.rs` RSS check via `sysinfo` crate (optional) — assert <100MB idle
- [ ] `.github/workflows/ci.yml` — matrix windows/linux/macos, clippy, test, build pure + full
- [ ] `cargo-dist` config + `README.md:20` install instructions, badges
- [ ] Golden tests: `tests/golden/mod.rs` insta for text/csv/office fixtures, `cargo insta review`

**Verify:** `cargo build --release` ~10MB, `--features full` ~25MB, `tui-preview --help` + `--preview` work.

## Phase 6 — Future Icebox (prioritized)

- [ ] Image zoom `+/−` + pan `hjkl` in fullscreen (`src/term/graphics.rs:80`)
- [ ] Slideshow `a` timer 2s (`src/app.rs:150` interval)
- [ ] XLSX formula toggle `f` show formula vs value
- [ ] Audio playlist queue `a` all mp3 in dir
- [ ] Video 5-thumb strip `t`
- [ ] File ops `d/r/n` delete/rename/new with confirm modal + `notify` watcher
- [ ] Yank `y` clipboard via `arboard` (pure Rust) — gated `clipboard` feature
- [ ] `--export-thumbs <outdir>` batch mode headless
- [ ] Lua/Rhai handler plugins post-v1

## Tech Debt & Hardening

- [ ] Add `cargo-deny` + `cargo-audit` to CI
- [ ] Fuzz handlers with `cargo-fuzz` on image/svg/pdf parsers (limit input 5MB)
- [ ] Bounded alloc: reject image >10000x10000, zip entries >10k, svg depth >100
- [ ] `tracing::instrument` on every handler `preview()` for debug.log timings
- [ ] Snapshot `fixtures` large files not committed LFS — keep <5MB each

## Definition of Done Global

For every phase: `cargo build` + `cargo build --features full` + `cargo clippy -- -D warnings` + `cargo test` + manual `cargo run -- ./fixtures` — all green before moving on.

## Daily Standup Template

```
Yesterday: completed TODO Phase X items …
Today: will do …
Blockers: needs FFmpeg libs? mupdf Windows build? ask.
```

Start at Phase 0 item 1 — `Cargo.toml:1` — and check off sequentially.


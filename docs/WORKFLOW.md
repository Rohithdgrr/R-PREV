# WORKFLOW.md — Development & Runtime Workflow

## 1. Development Workflow

### 1.1 Prerequisites

```powershell
winget install Rustlang.Rustup
rustup update stable
rustup component add clippy rustfmt
cargo install cargo-watch cargo-audit cargo-deny
# For --features video/pdf-raster (optional):
winget install LLVM  # for ffmpeg-next build
```

### 1.2 Project Bootstrap

```powershell
cd C:\Users\rohit\Music\VIJAY
cargo init --name tui-preview --bin
# Cargo.toml as per TECH-STACK.md
cargo build              # pure default, no C libs
cargo run -- .           # run against current dir
```

### 1.3 Daily Dev Loop

```powershell
# Auto-rebuild on save
cargo watch -x "run -- ."

# Fast checks
cargo clippy -- -D warnings
cargo fmt --check
cargo test --lib

# Before commit
cargo deny check
cargo audit
cargo test --all-features
```

### 1.4 Git Workflow

```powershell
git init
git add .
git commit -m "feat: scaffold ratatui app + image handler"
git branch -M main
# Conventional commits: feat, fix, docs, perf, refactor, test, chore
```

### 1.5 Code Review Checklist

- [ ] No `Command::new` (clippy disallowed)
- [ ] Handler returns `Result`, never panics on bad file
- [ ] Preview off main thread (`spawn_blocking`)
- [ ] Cache key includes mtime+size
- [ ] Added `insta` snapshot if UI text changed

## 2. Runtime Workflow — User Journey

### 2.1 Launch

```
User: tui-preview ./docs
  → App::new(path) -> list_dir(./docs) -> detect terminal cap -> enter ratatui loop
  → First file auto-preview (async)
```

```
┌──────────────────────┬────────────────────────────────────────┐
│ > docs/              │ Preview: README.md (Markdown)          │
│   ARCHITECTURE.md    │ # Architecture                         │
│   BACKEND.md         │                                        │
│   TECH-STACK.md      │  This project is a TUI orchestrator... │
│   WORKFLOW.md ◄────  │  (syntect highlighted)                 │
│   report.pdf         │                                        │
│   screenshot.png     │  2.1kB • 3 days ago • text/markdown   │
│                      │                                        │
│ [Normal] j/k nav  / search  p preview  f fullscreen  ? help  │
└──────────────────────┴────────────────────────────────────────┘
```

### 2.2 Navigation Workflow

```
j/k or Up/Down  → select next/prev → abort prior preview → spawn new preview
Enter           → if dir: cd into; if file: open fullscreen preview
Backspace       → go parent
/               → enter Search mode: nucleo-matcher filters list live
h               → toggle hidden files
g/G             → top/bottom
```

### 2.3 Preview Workflow per File Type

| User Action | Backend | UI |
|---|---|---|
| Select `photo.png` | ImageHandler → cache? → image::open → resize → term::graphics | Kitty/Sixel or half-block render |
| Select `report.pdf` | PdfHandler → lopdf text (instant) + mupdf raster (if feature) | Split: text left, image right |
| Select `sales.xlsx` | OfficeHandler calamine → first sheet Table | Comfy-table in Ratatui Table, Tab to next sheet |
| Select `demo.mp3` | AudioHandler lofty meta + symphonia waveform | Meta panel + Sparkline + Space to play via rodio |
| Select `demo.mp4` | VideoHandler ffmpeg thumbnail or mp4 meta | Thumbnail + ffprobe metadata |
| Select `main.rs` | TextHandler syntect | Syntax highlighted scroll |
| Select large `big.pdf` 120MB | Guard: TooLarge → Error pane "file >100MB, Enter to force" | User presses Enter → force decode |

### 2.4 Search Workflow

```
/ -> type "arch" -> nucleo-matcher scores ARCHITECTURE.md (1.0), BACKEND.md (0.2)
-> filtered list updates live, selected index reset to 0, preview follows
-> Esc clears search
-> n/N next/prev match (if many)
```

### 2.5 Audio Playback Workflow

```
Select song.mp3 -> AudioHandler shows waveform + Space hint
Press Space -> rodio Sink play (spawned thread, not blocking)
Press Space again -> pause
Press s -> stop
Selecting next file auto-stops prior Sink
```

### 2.6 Cache Workflow

```
Open image.png (first time) -> miss -> decode 280ms -> cache write async -> hit next time 18ms
~/.cache/tui-preview/thumbs/ab12...png created
Scrolling fast -> jobs aborted, only final selection completes (cancellation)
Cache eviction runs when folder >500MB -> delete oldest mtime files
```

## 3. Configuration Workflow

```powershell
# First run creates defaults
tui-preview --init-config   # writes ~/.config/tui-preview/config.toml
notepad ~/.config/tui-preview/config.toml
```

```toml
[general]
theme = "dark" # dark/light, truecolor auto
show_hidden = false
preview_delay_ms = 50 # debounce fast scroll

[cache]
max_disk_mb = 500
mem_entries = 100

[preview]
max_image_mb = 50
max_pdf_pages = 1 # v1 only first page

[keys]
quit = "q"
help = "?"
search = "/"
fullscreen = "f"
```

Changes hot-reload on next startup (watch with `notify` in future).

## 4. Build & Release Workflow

```powershell
# Debug
cargo run -- --help
cargo run -- ./fixtures

# Release pure
cargo build --release
.\target\release\tui-preview.exe --version

# Release full (needs FFmpeg/mupdf libs)
cargo build --release --features full

# Test
cargo test
cargo test --all-features

# Bench
cargo bench

# Release artifacts
cargo install cargo-dist
cargo dist build --artifacts
```

## 5. CI Workflow (Future .github/workflows/ci.yml)

```yaml
on: [push, pull_request]
jobs:
  check:
    runs-on: ${{ matrix.os }} # windows-latest, ubuntu-latest, macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo clippy -- -D warnings
      - run: cargo test --lib
      - run: cargo build --release # pure default
```

## 6. Troubleshooting Workflow

| Symptom | Check |
|---|---|
| Images show as blocks not graphics | Terminal caps: `echo $TERM`, try WezTerm/Kitty/Windows Terminal; fallback is expected in ConHost |
| PDF raster blank | Built without `--features pdf-raster`; text-only is default |
| Video thumbnails missing | Build with `--features video` and install FFmpeg |
| Slow preview | Check `~/.cache/tui-preview/debug.log`, clear cache `tui-preview --clear-cache` |
| Binary too large | Use `default` not `full`; ensure `strip=true` |

## 7. Performance Profiling Workflow

```powershell
cargo bench --bench preview
# Or ad-hoc:
Measure-Command { .\target\release\tui-preview.exe --bench ./fixtures --iterations 100 }
# Check RSS:
Get-Process tui-preview | Select-Object WS, CPU
```

Target budgets in ARCHITECTURE.md: enforce via criterion `assert!(duration < 300.ms)` in bench.

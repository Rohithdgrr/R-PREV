# USAGE.md — User Guide (Pure Rust tui-preview)

## 1. Install

### From Source (Pure Rust, no C libs)

```powershell
# Prereq: Rust stable
winget install Rustlang.Rustup

cargo install --path .   # default ~10MB, pure
# Or richer:
cargo install --path . --features pdf-raster   # + pdf images
cargo install --path . --features video        # + video thumbs (needs FFmpeg libs)
cargo install --path . --features full         # richest ~25MB

tui-preview --version
```

### Build Requirements for Features

| Feature | Needs | Windows Install |
|---|---|---|
| `default` | Nothing | — |
| `pdf-raster` | `mupdf` lib | `cargo build` fetches via crate, no extra step |
| `video` | FFmpeg 6.x libs | `winget install Gyan.FFmpeg` + LLVM, or `cargo xwin` cross |

If you see `ffmpeg-next build failed`, use `default` (metadata-only video) — works fine.

### Via cargo-dist Release (future)

```powershell
# After Phase 5 GitHub Release:
Invoke-WebRequest -Uri https://github.com/you/tui-preview/releases/latest/download/tui-preview-windows-x64.zip -OutFile tui-preview.zip
Expand-Archive tui-preview.zip
.\tui-preview.exe --help
```

## 2. Quick Start

```powershell
tui-preview                 # open current dir
tui-preview ./docs          # open dir
tui-preview ./report.pdf    # open and focus file
tui-preview --help
```

You see:

```
┌──────────────────────┬───────────────────────────────────┐
│ > myproject/         │ Preview: photo.png                │
│   photo.png  ◄────── │  [image: 4032×3024 PNG • 4.2MB]  │
│   report.pdf         │  (image rendered via Kitty/Sixel  │
│   sales.xlsx         │   or half-block fallback)          │
│   song.mp3           │                                   │
│   main.rs            │  4.2MB • 3 days ago               │
│                      │                                   │
│ [Normal] j/k nav  / search  f fullscreen  ? help  q quit │
└──────────────────────┴───────────────────────────────────┘
```

## 3. Keybinds

| Key | Action | Mode |
|---|---|---|
| `j` / `Down` | Next file | Normal |
| `k` / `Up` | Prev file | Normal |
| `g` | Top | Normal |
| `G` | Bottom | Normal |
| `Enter` | If dir → enter; if file → fullscreen preview | Normal |
| `Backspace` | Parent dir | Normal |
| `h` | Toggle hidden (dotfiles) | Normal |
| `/` | Search (fuzzy) | Normal → Search |
| `Esc` | Clear search / exit overlay | Search/Help/Fullscreen |
| `f` | Toggle fullscreen preview | Normal |
| `n` / `p` | Next/prev page (pdf/pptx) in fullscreen | Fullscreen |
| `Tab` / `Shift-Tab` | Next/prev sheet (xlsx) | Normal/Fullscreen |
| `Space` | Play/pause audio | Normal |
| `s` | Stop audio | Normal |
| `o` | Open with system app (`open::that`) | Normal |
| `y` | Yank path (future, `clipboard` feature) | Normal |
| `?` | Help overlay | Any |
| `q` | Quit (or `Ctrl-C`) | Any |

In **Search** mode, type to filter, `Enter` selects top match, `Esc` clears.

## 4. Preview by File Type

| File | What You See |
|---|---|
| `.png/.jpg/.webp/.svg` | Image via Kitty/Sixel or half-block + metadata |
| `.md` | Styled headings/lists/code (pulldown-cmark) |
| `.rs/.py/.js` | Syntax highlighted via syntect, line limit 5000 |
| `.csv/.tsv` | Table (100 rows, header frozen), delimiter auto |
| `directory` | Summary: `42 entries • 1.2GB` + largest files |
| `.pdf` | Text of first 2 pages + metadata; with `--features pdf-raster` split text+image at 150 DPI, `n/p` paginate |
| `.docx` | Paragraphs, headings, tables as text |
| `.xlsx` | First sheet Table, `Tab` switch sheets |
| `.pptx` | Slide titles+bullets paginated, `n/p` |
| `.mp3/.flac/.wav` | Tags + waveform Sparkline + `Space` play |
| `.mp4/.mkv` | Metadata; thumbnail if `--features video`, else hint |

Large files guard: `file >50MB image or >100MB pdf → "Press Enter to force preview"`.

## 5. CLI Flags

```powershell
tui-preview [PATH] [OPTIONS]

Options:
  --help            Show help
  --version         Version + enabled features + term caps
  --init-config     Write default ~/.config/tui-preview/config.toml
  --clear-cache     Delete ~/.cache/tui-preview then exit
  --preview <FILE>  Headless preview: print text/table or image escapes to stdout (for fzf)
  --theme <dark|light>  Override config.toml
  --bench <DIR>     Benchmark all files in dir, print timings

Examples:
  tui-preview ./fixtures --theme light
  tui-preview --clear-cache
  tui-preview --preview ./report.pdf | less -R
```

## 6. Config — `~/.config/tui-preview/config.toml`

Generate:

```powershell
tui-preview --init-config
notepad $env:APPDATA\tui-preview\config.toml  # Windows Path via directories crate
# Linux/macOS: ~/.config/tui-preview/config.toml
```

```toml
[general]
theme = "dark"          # dark | light
show_hidden = false
preview_delay_ms = 50   # debounce fast scroll (ms)

[cache]
max_disk_mb = 500
mem_entries = 100

[preview]
max_image_mb = 50
max_pdf_pages = 1        # v1 only first page
max_text_lines = 5000
max_text_bytes = 2097152 # 2MB

[keys]
quit = "q"
help = "?"
search = "/"
fullscreen = "f"
toggle_hidden = "h"
open_external = "o"

[ui]
truecolor = "auto"       # auto | always | never
```

Restart to apply. Env override: `TUI_PREVIEW_THEME=light tui-preview`.

## 7. fzf / Yazi Integration

### fzf Preview (after `--preview` flag Phase 5)

```powershell
# PowerShell:
fzf --preview 'tui-preview --preview {}' --preview-window=right:60%
# Bash:
fzf --preview 'tui-preview --preview {}'
```

### Yazi (future): add to `yazi.toml`:

```toml
[preview]
run = "tui-preview --preview \"$1\""
```

## 8. Automation / CI

```powershell
# Batch export thumbs without TUI (future --export-thumbs):
tui-preview --export-thumbs ./thumbs ./fixtures
# Then thumbs/ contains *.png thumbnails

# Benchmark:
tui-preview --bench ./fixtures --iterations 20
# Output: photo.png cold 240ms cached 18ms …
```

Best SSH workflow:

```powershell
ssh user@server
tui-preview ./ml-outputs   # previews generated charts over SSH, no X11
```

## 9. Troubleshooting

| Issue | Fix |
|---|---|
| Images show as `▀` blocks only | Terminal lacks Kitty/Sixel. Try WezTerm/Kitty/Ghostty/Windows Terminal 1.22+. Blocks are expected fallback in ConHost. |
| PDF only shows text, no image | Built without `pdf-raster`. Reinstall `cargo install --path . --features pdf-raster`. |
| Video thumbnails missing | `cargo install --path . --features video` required. Without, metadata only. |
| Preview says "Too large, Enter to force" | File exceeds `max_image_mb` in config. Press `Enter` or raise limit. |
| Cache grows | `tui-preview --clear-cache` or set `cache.max_disk_mb = 200`. |
| Slow on large XLSX | Sheet limited to 100 rows + "… +N rows". `Tab` to next sheet, not infinite scroll. |
| `ffmpeg-next build failed` on Windows | Use `cargo install --path .` (default, no FFmpeg). See TECH-STACK.md. |
| Colors wrong | `COLORTERM=truecolor` auto; try `--theme light` or set `ui.truecolor="never"`. |
| Debug | `RUST_LOG=debug tui-preview` then `cat ~/.cache/tui-preview/debug.log` |

## 10. Tips

- Press `?` anytime for bound keys + current terminal caps (shows `Kitty: yes/no` etc.).
- Press `f` to go fullscreen when image is small.
- Press `h` to find hidden `.env` files.
- Press `o` to open file in native app when you need full fidelity (video playback, Office edit).
- Keep `fixtures/` small for testing; cache is reused.

Enjoy fast, keyboard, pure-Rust preview — light, working, over SSH.


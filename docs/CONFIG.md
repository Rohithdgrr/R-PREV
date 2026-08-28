# CONFIG.md — Configuration Reference

## Location

`directories::ProjectDirs::from("com","tui-preview","tui-preview")` resolves:

| OS | Config Path | Cache Path |
|---|---|---|
| Windows | `%APPDATA%\tui-preview\config.toml` | `%LOCALAPPDATA%\tui-preview\cache` or `~/.cache/tui-preview` |
| Linux | `~/.config/tui-preview/config.toml` | `~/.cache/tui-preview` |
| macOS | `~/Library/Application Support/com.tui-preview.tui-preview/config.toml` | `~/Library/Caches/com.tui-preview.tui-preview` |

Generate defaults:

```powershell
tui-preview --init-config
notepad $env:APPDATA\tui-preview\config.toml
cat ~/.config/tui-preview/config.toml  # Linux
```

## Full Schema — `src/config.rs:1`

```toml
[general]
theme = "dark"          # "dark" | "light" | "auto"
show_hidden = false     # bool, h toggles at runtime
preview_delay_ms = 50   # u64, debounce fast scroll (0 = immediate)
follow_symlinks = true  # bool, false = show as link text

[cache]
max_disk_mb = 500       # u64, 0 = no disk cache
mem_entries = 100       # usize, 0 = no mem LRU
cache_dir = ""          # string, "" = default, else custom path

[preview]
max_image_mb = 50       # u64, guard threshold (see PERFORMANCE.md)
max_pdf_pages = 1       # usize, v1 only 1
max_text_lines = 5000   # usize, truncate beyond
max_text_bytes = 2097152 # u64, 2MB
max_xlsx_rows = 100     # usize per sheet view
thumbnail_size = 256    # u16, square thumb max dimension

[ui]
truecolor = "auto"      # "auto" | "always" | "never"
unicode = true          # bool, false forces ASCII fallback
status_hints = true     # bool, show j/k / etc in footer

[keys]
quit = "q"
help = "?"
search = "/"
fullscreen = "f"
toggle_hidden = "h"
open_external = "o"
parent = "Backspace"
play_pause = "Space"
stop = "s"
next_page = "n"
prev_page = "p"
next_sheet = "Tab"
prev_sheet = "Shift-Tab"
top = "g"
bottom = "G"
yank = "y"
sort = "s"

[handlers]
# Priority overrides (optional)
# image = 100
# pdf = 95
```

## Defaults if Missing

If file absent or partial, missing keys use `Default::default()` in `src/config.rs:40`:

```rust
#[derive(Deserialize)]
pub struct Config {
    #[serde(default)] pub general: General,
    #[serde(default)] pub cache: CacheCfg,
    // ...
}
```

Partial TOML is fine — only set what you override.

## Env Overrides

| Env | Overrides | Example |
|---|---|---|
| `TUI_PREVIEW_THEME` | `general.theme` | `TUI_PREVIEW_THEME=light tui-preview` |
| `TUI_PREVIEW_CACHE_DIR` | `cache.cache_dir` | `TUI_PREVIEW_CACHE_DIR=/tmp tui-preview` |
| `RUST_LOG` | tracing level | `RUST_LOG=debug tui-preview` |

Env wins over file.

## Key String Format

Keys are crossterm key strings parsed in `src/event.rs:60`:

- Single char: `q`, `/`, `h`
- Special: `Enter`, `Esc`, `Backspace`, `Tab`, `Space`, `Up`, `Down`
- With modifier: `Shift-Tab`, `Ctrl-C`, `Alt-F`

Invalid key strings log warning and fall back to default.

## Theme Details

- `dark`: syntect `base16-ocean.dark`, status `DarkGray`
- `light`: syntect `InspiredGitHub`, status `White`
- `auto`: checks `COLORFGBG` env (`;0` dark bg) else dark

Truecolor probe `COLORTERM=truecolor` else 256.

## Validation

On load, `Config::validate()` checks:

- `max_disk_mb > 0 && mem_entries >0` else warn and clamp
- `thumbnail_size 64..1024` else clamp
- Unknown keys ignored with `tracing::warn!("unknown config key: {}", k)`

Broken TOML → log `warn!("config parse failed, using defaults: {}", e)` and continue, never crash.

## Hot Reload (Future)

`notify` watches config file; on change, `app.config = Arc::new(new)` and re-render. v1 requires restart; doc marks future.

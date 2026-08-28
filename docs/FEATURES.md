# FEATURES.md — Native App Killer Catalog — v2

> ✅ = in default build, 🔜 = behind feature or next phase, 💡 = post-v2. New v2 tags: ★ killer over native apps.

## 1. Core TUI

| Feature | Desc | Phase |
|---|---|---|
| Two-pane 30/70 + footer + modals | — | ✅ |
| `j/k g/G h/l` nav | vim | ✅ |
| fuzzy `/` `nucleo-matcher` | live | ✅ |
| fullscreen `f` | — | ✅ |
| help `?` | cheatsheet + term caps | ✅ |
| theme dark/light | + auto truecolor | ✅ |
| hidden `h` parent `Backspace` | — | ✅ |
| **Daemon `<5ms` hot startup** ★ | `--daemon` socket, client IPC `interprocess`, auto-start if missing, `systemd/launchd` unit | 🔜 `daemon` |
| **Redux time-travel** ★ | `Store<State,Action,Effect>` `Ctrl+Shift+T` step back, deterministic replay file for bug reports | 🔜 |
| **Wasm plugins** ★ | `*.wasm` in `~/.config/tui-preview/plugins`, Python/JS/Rust via `extism`/`wasmtime`, hot-reload, trap-isolated, WIT `preview(path,area)->CBOR` | 🔜 `wasm` |
| mouse `scroll/click/drag` ★ | `crossterm EnableMouseCapture` | 🔜 |
| clipboard `Ctrl+C` ★ | `arboard` image/text → desktop bridge | 🔜 `clipboard` |
| yank `y` | path | 🔜 `clipboard` |
| sort `s` bookmarks `m` `'` | — | 💡 |
| file ops `d r n` | modal confirm | 💡 |

## 2. Image ★

| Format | Handler | Phase |
|---|---|---|
| png/jpg/gif/webp/bmp | `image` Lanczos3 | ✅ |
| svg | `resvg`+`usvg` | ✅ |
| **EXIF panel `x`** ★ | `little_exif` GPS/camera/lens | 🔜 `exif` |
| ico/tiff | `image` | 🔜 |
| Kitty/Sixel/iTerm2/half-block `▀` | always works | ✅ |
| zoom `+/-` pan `hjkl` | fullscreen | 🔜 |
| slideshow `a` 2s | — | 🔜 |

`4032×3024 PNG 4.2MB EXIF: iPhone` bar via `src/preview/meta.rs`

## 3. Text / Code ★

| Feature | Phase | Killer over native |
|---|---|---|
| **tree-sitter AST** `tree-sitter` folding `z` jump `gd` scope | 🔜 `tree-sitter` | syntect regex → real parse (Sublime/VS Code parity) |
| syntect fallback | ✅ | — |
| **lazy viewport** `SparseIndex` `memmap2` only `height` lines ★ | ✅ | 1GB log `<50ms` not `2s` |
| **simd-json / simdutf8** for `.json`/ndjson | 🔜 `simd` | 3-10× serde |
| binary guard `content_inspector` | ✅ | hex fallback |
| encoding `encoding_rs` | ✅ | — |
| **hex editor `H`** ★ | 🔜 `hex` | `hexyl`-style dump + ascii, power inspect |
| markdown `pulldown-cmark` | ✅ | — |
| csv `csv` 100 rows + `SparseIndex` | ✅ | viewport only |
| **io_uring** dir walk `tokio-uring` (Linux NVMe) | 🔜 `io-uring` | -30% latency |
| log tail `T` | 💡 | `notify` watch |
| line nos `L` in-file `Ctrl-F` | 🔜 | — |

## 4. PDF ★

| Feature | Phase |
|---|---|
| text `lopdf`+`pdf-extract` 2 pages searchable | ✅ |
| count/author | ✅ |
| raster `pdfium-render` Apache-2.0 **child-isolated** `n/p` paginate | 🔜 `pdf-raster` |
| **in-doc search `Ctrl-F`** ★ | 🔜 |
| AI summarize ★ | 🔜 `local-ai` |
| strip future | 💡 |

`mupdf` AGPL removed.

## 5. Office / Archive ★

| Format | Handler | Phase |
|---|---|---|
| docx `docx-rs` | paragraphs/tables | ✅ |
| xlsx `calamine` `Tab` sheets | ✅ | 
| pptx `zip`+`quick-xml` (was `pptx-rs` abandoned) `n/p` | ✅ |
| **archive VFS ★** zip/tar/tar.gz/7z as folder `Enter` dives **no extract** | 🔜 | killer over Explorer zip preview |
| archive listing `Name Size Packed Ratio` via `zip`/`tar`/`sevenz-rust` | ✅ listing; VFS 🔜 `archive-vfs` |
| `du` on demand `D` async cache | 🔜 |
| xlsx formula `f` toggle | 🔜 |

## 6. Audio ★ true playback

| Feature | Phase |
|---|---|
| mp3/flac/wav/ogg/m4a `symphonia` `lofty` | ✅ |
| waveform 30s 80-bar sparkline | ✅ |
| **playback `Space` rodio Sink background** ★ | ✅ (was meta only) |
| volume `-=` seek `<>` 5s | 🔜 |
| playlist queue `a` | 💡 |

## 7. Video ★ flipbook

| Feature | Phase |
|---|---|
| meta `mp4` header | ✅ default |
| **thumbnail child-isolated** `ffmpeg-next` frame 10% | 🔜 `video` (was segfault-risk) |
| **flipbook playback** ★ Kitty/Sixel frames at 15fps while focused | 🔜 `video` |
| strip `t` 5 thumbs | 💡 |
| `o` mpv `e` $EDITOR | ✅ |

## 8. Filesystem ★

| Feature | Phase |
|---|---|
| dir summary `42 entries 1.2GB` largest | ✅ |
| `ignore` .gitignore | ✅ |
| symlink `→` depth 10 red broken | ✅ |
| `rwx` `4.2M` | ✅ |
| **git badges** `M ? S` `gix` + **blame `B` diff `d` history** ★ | 🔜 `git` |
| `notify` watch `watch` | 🔜 |
| **archive VFS deep nav** ★ | 🔜 `archive-vfs` |

## 9. Cache / Perf killer

| Feature | Phase |
|---|---|
| two-tier 100 + 500MB | ✅ |
| **quantized 8×4** no churn | ✅ |
| **timeout 5s centralized + Wasm fuel + child kill** | ✅ |
| **sized pool `(num_cpus/2).clamp`** | ✅ |
| **memmap2 + SparseIndex lazy** | ✅ |
| **simd / io_uring opts** | 🔜 |
| `panic=unwind` kept | ✅ |
| `Cargo.lock` committed | ✅ |
| bench regression gate | 🔜 CI |

## 10. Config

| Key | Desc |
|---|---|
| `theme show_hidden preview_delay_ms` | ✅ |
| `max_disk_mb mem_entries worker_threads` | ✅ |
| `keys` remap | ✅ |
| `truecolor` | ✅ |
| `--init-config` | ✅ |
| `daemon.socket idle_timeout_secs` | 🔜 |
| `plugins.dir hot_reload fuel` | 🔜 wasm |
| `store.history_size` | 🔜 |
| `sandbox.strict` | 🔜 |
| `ai.model` | 🔜 local-ai |
| `TUI_PREVIEW_THEME` env | 🔜 |

## 11. Integration ★

| Feature | Phase |
|---|---|
| `tui-preview [PATH]` cold/hot daemon aware | ✅ |
| `--preview <file>` fzf headless CBOR/text | 🔜 |
| **fzf `fzf --preview 'tui-preview --preview {}'`** | 🔜 |
| **yazi/ranger `preview` subcommand** | 🔜 |
| **`--export-thumbs` + `--preview` + `--bench` CI batch** | 🔜 |
| **hex `H` + arboard copy + mouse bridge** | 🔜 |
| **`$EDITOR e`** text jump | 🔜 vs `o` mpv |
| **local AI `candle` summarize/explain/semantic search** ★ | 🔜 `local-ai` |
| SSH headless Sixel degrade | ✅ |

## 12. Polish

| Feature | Phase |
|---|---|
| truecolor/unicode | ✅ |
| Error never crash — child/Wasm trap → red pane | ✅ |
| debug.log + Redux event replay file | ✅ |
| help `?` caps + time-travel `Ctrl+Shift+T` | 🔜 |
| version `V` features | ✅ |

## 13. Flags v2

| Flag | Adds | Size |
|---|---|---|
| `default` | ~10MB pure | 10 |
| `pdf-raster` | pdfium Apache child-isolate | +5 |
| `video` | ffmpeg child-isolate | +15 |
| `watch` git | notify, gix |  |
| `daemon` | interprocess `<5ms` | +1 |
| `wasm` / `wasm-extism` | wasmtime/extism plugins | +3 |
| `tree-sitter` | AST folding/gd | +2 |
| `simd` | simd-json/utf8 |  |
| `io-uring` | tokio-uring Linux |  |
| `local-ai` | candle local LLM | +20 |
| `clipboard` | arboard |  |
| `archive-vfs` | sevenz |  |
| `hex` `sandbox` `exif` | — |  |
| `full` | all above minus ai/uring/sandbox | ~25 |
| `full-ai` | + ai+uring+sandbox | ~45 |

## 14. Non-features

Video smooth 60fps (flipbook 15fps), doc edit, PDF annotate >150DPI, Office pixel layout, DRM.

## 15. Graph v2

```
Store(Redux)+Daemon(hot)+Router(5s+child+Wasm) must
  ├─ Image+EXIF
  ├─ Text tree-sitter lazy + simd json
  ├─ Archive VFS
  ├─ Pdf child-isolated
  ├─ Office zip+quick-xml
  ├─ Audio rodio play
  ├─ Video flipbook child
  ├─ Hex editor
  ├─ AI semantic
Cache quantized + sized + memmap2
Term mouse+clipboard half-block
FS VFS + git blame + du
```

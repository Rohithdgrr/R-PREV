# TECH-STACK.md — Pure Rust Stack — Native App Killer v2

> **Core stays pure Rust single-binary.** v2 adds are **additive optional features** — daemon, Wasm, tree-sitter, SIMD, io_uring, local AI, hex, mouse/clipboard. Default build still ~10 MB pure; `full` + all opts ~35 MB. Bad deps still banned (AGPL).

## 1. Stack Philosophy — v2

- **Zero shell-out core** kept + daemon IPC via `interprocess` (pure Rust, no `Command` of external binaries).
- **Instant:** daemon `<5ms` hot vs `60ms` cold — pre-warmed `Cache`/`SyntaxSet`/`WasmStore`.
- **Extensible without recompile:** Wasm (`wasmtime`/`extism`) sandbox — user `.wasm` plugins in Python/JS/Rust, trap-isolated.
- **Intelligent:** `tree-sitter` AST vs `syntect` regex; `simd-json`/`simdutf8` vs `serde_json`.
- **Stable:** `catch_unwind` for Rust + **child-process isolation for C** (`pdfium`/`ffmpeg` segfault → parent survives).
- **Lean default:** trimmed `tokio` (`rt, rt-multi-thread, macros, time, sync, fs`), `panic=unwind` kept, `full` pulls heavy opts only when asked.

## 2. Core Dependencies — `Cargo.toml:1` — delta vs v1

### 2.1 Runtime & UI — + daemon IPC + Redux + mouse

| Crate | Ver | Role | Feature |
|---|---|---|---|
| `ratatui` | 0.29 | TUI | default |
| `crossterm` | 0.28 + `event-stream` | I/O + mouse `EnableMouseCapture` | default |
| `tokio` | `rt, rt-multi-thread, macros, time, sync, fs` | trimmed runtime — was `full` (fixed) | default |
| `interprocess` | 2.2 | Unix socket / Named Pipe IPC daemon/client | `daemon` |
| `serde` + `toml` + `rmp-serde` | 1 / 0.8 / 1 | config + IPC CBOR | default |
| `num_cpus` | 1.16 | pool `(get/2).clamp(2,6)` | default |
| `thiserror` | 2 | typed errors | default |
| `clap` | 4.5 `derive` | `tui-preview [--daemon| PATH| --isolated-child]` | default |
| `tracing` | 0.1 | file log `~/.cache/tui-preview/debug.log` + Redux event log | default |
| `arboard` | 3.4 | clipboard `set_text`/`set_image` (x11/wayland/win/macos) | `clipboard` |
| `directories-next` | 2 | cache/config dirs (Linux XDG) | default |

### 2.2 Filesystem — + VFS + io_uring

| Crate | Role | Feature |
|---|---|---|
| `walkdir` 2 + `ignore` 0.4 | listing | default |
| `mime_guess` 2 + `infer` 0.19 | MIME + magic | default |
| `nucleo-matcher` 0.3 | fzf-like search | default |
| `notify` 6 **optional** | watcher | `watch` (fixed) |
| `memmap2` 0.9 | zero-copy large text | default |
| `tokio-uring` 0.5 | Linux `io_uring` dir walk + mmap prefault zero-copy NVMe | `io-uring` (Linux only) |
| `sha2` + `hex` + `lru` | cache key + LRU | default |
| `gix` 0.66 optional | git badges/blame/diff | `git` |
| `sevenz-rust` 0.6 optional | 7z VFS read | `archive-vfs` |
| `tar` 0.4 + `flate2` 1 + `zip` 2 + `quick-xml` 0.36 | archive VFS | default (tar/zip) |

### 2.3 Text / Code — + simd + tree-sitter + lazy viewport

| Crate | Role | Feature | Replaces |
|---|---|---|---|
| `syntect` 5 | regex highlight fallback | default | — |
| `tree-sitter` 0.24 + `tree-sitter-rust` etc | AST highlight, folding, `gd` jump | `tree-sitter` | upgrades syntect |
| `simd-json` 0.14 + `simdutf8` 0.1 | 3-10× JSON/log parse, utf8 validate | `simd` | upgrades serde_json |
| `csv` 1 + `comfy-table` 7 | CSV table + `SparseIndex` lazy viewport | default | — |
| `pulldown-cmark` 0.12 | markdown | default | — |
| `encoding_rs` 0.8 + `content_inspector` 0.2 | charset + binary guard | default | — |
| `memmap2` + `memchr` 2 | sparse line index `offsets: Vec<usize>` → only viewport decoded | default | — |
| `hexyl` 0.12 style (`hex` crate) | hex dump view read-only | `hex` feature | — |

### 2.4 Image — + EXIF

| Crate | Role |
|---|---|
| `image` 0.25 + `resvg` 0.43 `usvg` | raster + SVG |
| `little_exif` / `kamadak-exif` 0.3 | EXIF panel `x` (GPS, camera) — `exif` feature |

### 2.5 PDF — fix kept: pdfium (Apache-2.0)

| Crate | Role | Feature |
|---|---|---|
| `lopdf` 0.35 + `pdf-extract` 0.8 | text | default |
| `pdfium-render` 0.8 (Apache-2.0) | raster 150 DPI **child-isolated** | `pdf-raster` (was `mupdf` AGPL — removed) |

### 2.6 Office/Archive

| Crate | Role |
|---|---|
| `docx-rs` 0.4 + `calamine` 0.26 + `zip` + `quick-xml` | docx/xlsx + in-house pptx (was `pptx-rs` abandoned) |
| `zip` + `tar`/`flate2` + `sevenz-rust` optional | archive listing + VFS without extract |

### 2.7 Audio/Video — isolated

| Crate | Role | Feature | Isolation |
|---|---|---|---|
| `symphonia` 0.5 + `lofty` 0.22 + `rodio` 0.20 | audio meta+play | default | in-process (pure Rust, no segfault) |
| `mp4` 0.14 | video header | default | in-process |
| `ffmpeg-next` 7 | video thumbnail + flipbook frames | `video` | **child-isolated** (`--isolated-child`) |

### 2.8 Wasm Plugin Runtime ★ NEW

| Crate | Role | Feature | Benefit |
|---|---|---|---|
| `wasmtime` 32 + `wit-component` | Wasm execute Python/JS/Rust `.wasm` previewers, WIT `preview(path,area)->CBOR`, fuel+epoch timeout | `wasm` | recompile-free ext, trap-isolated |
| `extism` 1 alternative | higher-level plugin SDK, PDK for Python/JS | `wasm-extism` | simpler authoring |

WIT: `package tui:preview; interface preview { preview: func(path: string, area: record {w: u32, h: u32}) -> result<bytes, string> }` — bytes = CBOR `PreviewResult`.

### 2.9 AI / Semantic (optional) ★ NEW

| Crate | Role | Feature |
|---|---|---|
| `candle-core` + `candle-transformers` | local LLM/CPU summarize `summarize this PDF` | `local-ai` |
| `llama-cpp-rs` alt | GGUF `mistral` local | `local-ai-llama` |
| `hnswlib`/`instant-distance` | semantic embedding cosine search across dir | `local-ai` |

Model `~/.cache/tui-preview/models/mistral.gguf` lazy download, CPU only default (GPU via `candle-cuda` optional).

### 2.10 Security / Sandboxing ★ NEW

| Crate | Role | Feature |
|---|---|---|
| `landlock` 0.4 | Linux Landlock restrict worker `READ(cache) WRITE(thumbs)` | `sandbox` (Linux) |
| `cargo-fuzz` + `libFuzzer` | `fuzz/fuzz_targets/*.rs` per handler | dev `cargo fuzz` |
| `wasmtime` fuel | Wasm timeout trap (no thread block) | `wasm` |

### 2.11 Dev/Quality

| Tool | Role |
|---|---|
| `clippy` `disallowed_methods` | bans `Command::new` except `open` + `isolated` child |
| `cargo-deny` | bans `GPL/AGPL` — caught `mupdf` |
| `cargo-audit` + `cargo bench` regression gate | CI |

## 3. Feature Flags — v2 matrix

```toml
[features]
default = []                          # ~10 MB base: pure, syntect, simd off, no daemon, no wasm
pdf-raster = ["pdfium-render"]        # +5 MB, Apache-2.0, child-isolated
video = ["ffmpeg-next"]               # +15 MB, child-isolated
video-pure = ["mp4"]
watch = ["notify"]
git = ["gix"]
daemon = ["interprocess"]             # daemon/client IPC
wasm = ["wasmtime", "wit-component"]   # or wasm-extism
wasm-extism = ["extism"]
tree-sitter = ["tree-sitter", "tree-sitter-rust", "tree-sitter-python"] # AST
simd = ["simd-json", "simdutf8"]       # JSON/log fast path
io-uring = ["tokio-uring"]            # Linux NVMe
local-ai = ["candle-core", "candle-transformers"] # local LLM
hex = []                              # hex view (built-in)
clipboard = ["arboard"]
archive-vfs = ["sevenz-rust"]         # +7z
sandbox = ["landlock"]               # OS sandbox (Linux)
full = ["pdf-raster", "video", "watch", "git", "daemon", "wasm", "tree-sitter", "simd", "clipboard", "archive-vfs", "hex"]
full-ai = ["full", "local-ai", "io-uring", "sandbox"]
```

## 4. Why These Upgrades (benchmarks)

| Replaces | With | Gain | Cost |
|---|---|---|---|
| cold start 60ms | daemon hot <5ms | instant-native | 4 MB daemon memory |
| syntect regex | tree-sitter AST | folding, `gd`, scope | larger grammars |
| serde_json | simd-json | 3-10× 10MB JSON | simd feature |
| read+copy text | memmap2 + SparseIndex lazy | O(viewport) not O(file) | index build 10ms/GB |
| tokio::fs | tokio-uring (Linux) | -30% NVMe latency | Linux only |
| handler recompile | Wasm plugin | no rebuild, trap safe | wasmtime ~3 MB |
| catch_unwind-only C | child isolation | segfault → parent alive | fork 15 MB ephemeral |
| unveil none | Landlock/Seatbelt | least-privilege per worker | kernel 5.13+ |
| manual repro | Redux history | deterministic replay | 5 MB ring |

## 5. Build & Verify

```powershell
cargo build --release                              # ~10 MB pure
cargo build --release --features daemon,wasm,tree-sitter,simd,clipboard  # ~20 MB
cargo build --release --features full-ai           # ~35 MB heaviest
cargo clippy -- -D warnings
cargo deny check   # bans AGPL — was catching mupdf
cargo fuzz list
cargo test --all-features
```

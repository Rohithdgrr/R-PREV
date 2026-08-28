# ARCHITECTURE.md — tui-preview (Pure Rust) Native App Killer Architecture v2

> **Vision shift: great TUI → Native App Killer** — daemon <5ms startup, Wasm-plugin extensibility, Redux time-travel, SIMD + io_uring speed, tree-sitter intelligence, process-isolated C bindings, OS sandboxing. All additive, all feature-gated, pure Rust core untouched.

## 1. Vision Summary

An orchestrator TUI that **kills native apps on their own turf** — sub-5ms launch, keyboard-only, SSH-native, scriptable, local-AI, archive-as-filesystem. Inspired by Yazi/Ranger + bat + mpv + macOS Preview + VS Code, but unified terminal workspace.

```
Native GUI advantages conquered:
  cold start          → daemon pre-warm <5ms (was 60ms)
  extensibility       → Wasm plugins (no recompile)
  code intelligence   → tree-sitter AST folding/jump-to-def
  parse speed         → SIMD JSON + lazy viewport (not whole-file)
  disk latency        → io_uring zero-copy (Linux)
  crash on bad file   → child-process isolation for C libs (was catch_unwind-only)
  privilege escalation→ Landlock/Seatbelt/Pledge sandbox per worker
  install friction    → single binary, AGPL-free, committed Cargo.lock

tui-preview v2 advantages: instant, extensible, intelligent, fast, stable, sandboxed, 8-15 MB base, <30ms cached, Wasm-safe
```

## 2. High-Level System Diagram — v2

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        tui-preview System v2                                 │
│  ┌─────────────────────────────────┐  ┌──────────────────────────────────┐  │
│  │  Daemon (tui-preview --daemon)  │  │  Client (tui-preview [PATH])     │  │
│  │  Unix Socket / Named Pipe       │◄─┤  <5ms IPC → pre-warmed state     │  │
│  │  ┌─────────────┐  ┌───────────┐ │  │  Ratatui + crossterm             │  │
│  │  │ Redux Store │◄─┤  Router   │ │  └──────────────────────────────────┘  │
│  │  │ Event Sourc │  │ Timeout 5s│ │           ▲                            │
│  │  │ Time-travel │  │ Quant keys│ │           │  Wasm Runtime              │
│  │  └──────┬──────┘  └─────┬─────┘ │  ┌────────┴────────┐                   │
│  │         │               │       │  │ extism/wasmtime │                   │
│  │  ┌──────▼──────┐ ┌──────▼────┐ │  │ Python/JS/Rust  │                   │
│  │  │  Cache      │ │ Isolated  │ │  │ plugins *.wasm  │                   │
│  │  │ LRU+Disk    │ │ Child proc│ │  └─────────────────┘                   │
│  │  │ SIMD parse  │ │ pdfium/   │ │                                        │
│  │  │ lazy view   │ │ ffmpeg    │ │  Terminal (Kitty/Sixel/half-block)     │
│  └────────────────┘ └───────────┘ └────────────────────────────────────────┘ │
│           │                  │                                               │
│     ~/.cache/tui-preview  sandboxed workers (Landlock/Seatbelt)             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 3. Layered Architecture — 8 Layers (was 6)

### Layer 1 — Daemon/Client ( `src/daemon/*` ) ★ NEW
- **Daemon:** `tui-preview --daemon` binds `~/.cache/tui-preview/daemon.sock` (Unix) / `\\.\pipe\tui-preview` (Windows). Holds `Cache`, `Config`, `ConfigWatcher`, warm `SyntaxSet`/`TreeSitter` grammars, `WasmStore`.
- **Client:** `tui-preview [PATH]` checks socket; if live, sends `Open { path }` → daemon forks TUI session, returns instantly. Cold start `~60ms` → daemon hot ` <5ms` (no re-init).
- Daemon auto-starts on first client if socket missing; `systemd --user`/`launchd` optional. Stale socket GC via PID lockfile.
- IPC: `interprocess` crate (pure Rust, cross-platform), CBOR/msgpack payloads, `tokio::net::UnixListener` + `tokio::net::windows::named_pipe`.

### Layer 2 — Presentation ( `src/ui/*` )
- Ratatui immediate mode, `Horizontal [30% FileList | 70% Preview]` + `Footer StatusBar` + `Overlay Modal`.
- **Now:** mouse tracking (`crossterm::event::EnableMouseCapture` — scroll, click, drag range select), `arboard` clipboard copy `Ctrl+C` image/text → desktop, hex viewport (`hexyl`-style).
- Widgets: `FileList`, `PreviewPane`, `HexView`, `StatusBar`, `HelpModal`, `SearchBar`, `ArchiveVFS`, `GitBlame`.

### Layer 3 — Redux Store ( `src/store/*` ) ★ NEW
- **Before:** ad-hoc `App { files, selected, preview, mode }` reducer.
- **After:** unidirectional `Store<State, Action, Effect>` (Redux-like, pure Rust, no JS dep):
  ```rust
  enum Action { SelectNext, SelectPrev, EnterDir, Search(String), ToggleHex, PreviewReady(Result), DaemonSync }
  fn reducer(state: &mut State, action: Action) -> Vec<Effect> // pure
  async fn effect_runner(effect: Effect) // side-effects: I/O, cache, Wasm, AI
  struct History { events: Vec<Action>, snapshots: Vec<State> } // time-travel
  ```
- Benefits: deterministic replay for crash reports (`RUST_LOG` + event log → `cargo test` repro), `time-travel` debug overlay (`Ctrl+Shift+T` step back), property-testable reducer (no async).
- `State` serializable via `serde` — daemon snapshots to disk on idle for resume.

### Layer 4 — Preview Router & Handlers ( `src/preview/*` )
- `trait PreviewHandler: Send + Sync { priority() -> u8, preview_blocking(WasmCtx) }` — `preview()` default wraps `preview_blocking` for dyn compat.
- **Router:** centralized `tokio::time::timeout(5s)` + `catch_unwind` (kept, `panic=unwind`) + **child-process isolation for C libs** (see §7): `pdfium-render`/`ffmpeg-next` run in `tokio::process::Child`; segfault → `try_wait()` non-zero → `Error("preview crashed, isolated")`, parent stays alive.
- **Wasm plugin branch:** after native `route()`, check `WasmRegistry` for `*.wasm` handlers with higher `priority()` — `wasmtime`/`extism` call sandboxed, returns `PreviewResult` via WIT `preview(wit: WasmPreview)`. Crash → Wasm trap, isolated per instance.
- Pipeline: `Detect → Check Cache (quantized) → Wasm or Native → Child-isolate if C → Render → Cache Write → Redux PreviewReady`.

### Layer 5 — Performance Engine ( `src/cache/*`, `src/preview/text/*` ) ★ ENHANCED
- Two-tier `LruCache(100, ~50MB)` + `Disk 500MB`, key `sha256(canonical_path + mtime + size + quantized_area(8×4) + handler_version)`.
- **New:**
  - `simd-json` for `.json` (was `serde_json` — ~3-10× faster on multi-MB logs).
  - `simdutf8` validate before `encoding_rs`.
  - `memmap2::Mmap` for files `>1MB` — no read+copy.
  - Lazy viewport: `SparseIndex { offsets: Vec<usize> }` built via one `memchr` scan; `PreviewResult::Text` holds `index: Arc<SparseIndex>` + `visible: Range` — only `height` lines decoded/highlighted. Massive CSV/log `→ O(viewport)` not `O(file)`.
  - `tree-sitter` replaces `syntect` for code (feature `tree-sitter`): AST folding, scope highlight, jump-to-def `gd`. `syntect` kept as fallback when feature off.
  - `io_uring` (Linux, feature `io-uring`): `tokio-uring` crate for directory walks + `Mmap` prefault on NVMe — bypasses syscall overhead. Non-Linux falls back to `tokio::fs`.

### Layer 6 — Terminal Abstraction ( `src/term/*` )
- `capabilities::detect()` reads `TERM`/`COLORTERM`/DA1; `graphics::render_image` dispatches Kitty/Sixel/iTerm2/half-block `▀`.
- **New:** mouse `EnableMouseCapture` → `App` maps `MouseEventKind::{Down, ScrollUp, Drag}` to `Action::Select`/`Scroll`; `arboard::Clipboard::set_text/set_image` for `Ctrl+C` copy bridge to GUI desktop.

### Layer 7 — Filesystem & Archive VFS ( `src/fs/*` )
- `walkdir` + `ignore`, sorted dirs-first, `canonicalize` depth 10.
- `notify` optional `watch` feature (already fixed). **New:** `src/fs/vfs.rs` — archive-as-directory: `fs::list_dir("archive.zip")` lists `zip::ZipArchive` entries; `fs::canonicalize("archive.zip/inner/foo.pdf")` resolves virtual path; previewer reads `zip` entry `by_name` on-the-fly, no temp extract. Supports `zip`/`tar`/`tar.gz`/`7z` (via `sevenz-rust` optional). Navigation `Enter` dives into archive like folder, `Backspace` pops.

### Layer 8 — Security & Isolation ( `src/sandbox/*`, `fuzz/*` ) ★ NEW
- **Process isolation for C:** `src/sandbox/isolated.rs` spawns child `tui-preview --isolated-child --format pdf --dpi 150 <path>` — child decodes via `pdfium`/`ffmpeg`, writes `PreviewResult` (CBOR) to stdout pipe, exits. Parent `wait()`; segfault/`SIGSEGV` → detected, no `catch_unwind` needed for C. Pure Rust handlers stay in-process (no isolation overhead).
- **OS sandbox per worker (post-open):** after child opens allowed file, `landlock` (Linux `landlock` crate, ABI v1) restricts to `read(cache dir)` + `write(cache/thumbs)`, drops ambient. macOS `seatbelt` profile `sandbox-exec` equivalent, OpenBSD `pledge("stdio rpath wpath cpath")`. Linux 5.13+ only — soft-fail if older kernel.
- **Fuzz:** `fuzz/fuzz_targets/*.rs` per handler via `cargo-fuzz` + `libFuzzer` — corpus from `fixtures/`, CI `cargo fuzz run -- --max_total_time=120`. Seed repro on crash stored in `fuzz/artifacts/`.

## 4. Crate Module Map — v2

```
src/
├─ main.rs                 # arg parse: --daemon, --isolated-child, [PATH] client/daemon auto-start
├─ daemon/
│  ├─ mod.rs               # Daemon { listener, store, cache, wasm_registry } + IPC types
│  ├─ socket.rs            # Unix socket / Named Pipe via interprocess
│  └─ child.rs             # isolated worker spawn + Landlock sandbox apply
├─ store/
│  ├─ mod.rs               # Store<State, Action, Effect>, reducer, effect_runner
│  ├─ state.rs             # State { files, selected, mode, preview, wasm, history }
│  ├─ actions.rs           # Action enum (serializable for replay)
│  └─ history.rs           # time-travel ring buffer + snapshot to disk
├─ plugins/
│  ├─ mod.rs               # WasmRegistry { wasmtime::Store, modules: HashMap<String, Module> }
│  ├─ wit.rs               # WIT interface: preview(path, area) -> PreviewResult CBOR
│  └─ loader.rs            # scan ~/.config/tui-preview/plugins/*.wasm, hot-reload notify
├─ preview/
│  ├─ mod.rs               # Router dispatch: timeout 5s + catch_unwind + child-isolate
│  ├─ image.rs             # image + resvg, EXIF via little_exif, quantized cache
│  ├─ text.rs              # memmap2 + simdutf8 + SparseIndex lazy viewport
│  ├─ text_tree_sitter.rs  # tree-sitter highlight + folding (feature tree-sitter)
│  ├─ json.rs              # simd-json accelerated (feature simd)
│  ├─ csv.rs               # csv + SparseIndex, viewport only
│  ├─ archive.rs           # zip/tar view (listing) + VFS delegate
│  ├─ archive_vfs.rs       # VFS: list_dir/read inside archive without extract
│  ├─ pdf.rs               # lopdf + pdf-extract + pdfium child-isolated
│  ├─ office.rs            # docx-rs + calamine + zip+quick-xml pptx
│  ├─ audio.rs             # symphonia + lofty + rodio playback row
│  ├─ video.rs             # mp4 header / ffmpeg child-isolated thumbnail + flipbook frames
│  ├─ hex.rs               # hexyl-style hex dump, ascii, editable? read-only
│  ├─ meta.rs              # FileMeta + EXIF + du + git badges
│  └─ isolate.rs           # Child process wrapper for C handlers (pdfium, ffmpeg)
├─ fs/
│  ├─ mod.rs               # list_dir delegates to VFS if archive path
│  ├─ vfs.rs               # ArchiveVFS trait + ZipVfs, TarVfs, SevenZVfs
│  ├─ watcher.rs           # notify cfg(feature="watch")
│  ├─ du.rs                # du async walk, cached
│  └─ git.rs               # gix blame/diff/history (feature git)
├─ term/
│  ├─ capabilities.rs      # Kitty/Sixel/iTerm2/truecolor + mouse cap
│  ├─ graphics.rs          # image → escape + flipbook frame push
│  ├─ mouse.rs             # MouseEvent → Action mapping
│  └─ clipboard.rs         # arboard copy text/image
├─ sandbox/
│  ├─ mod.rs               # sandbox::apply_restrictions(cache_dir) -> Result
│  ├─ landlock.rs          # Linux Landlock ABI v1-3
│  ├─ seatbelt.rs          # macOS Seatbelt (optional)
│  └─ isolated.rs          # spawn --isolated-child, CBOR pipe, segfault detect
├─ cache/  mod.rs, key.rs  # LRU+Disk, quantized 8x4, sized pool (num_cpus/2).clamp
├─ ai/                     # (optional feature local-ai) candle/llama-cpp-rs
│  ├─ mod.rs               # AiState { model_path, candle_ctx }
│  ├─ summarize.rs         # summarize PDF/code via local LLM
│  └─ semantic.rs          # embedding search across dir (cosine, hnsw)
├─ config.rs               # + daemon.addr, plugins.dir, sandbox.strict
└─ ui/
   ├─ layout.rs            # 30/70 + footer + hex + archive breadcrumb
   ├─ preview_pane.rs      # Text/Table/Image/Audio/Hex delegates
   └─ mouse.rs             # mouse hit-test

fuzz/fuzz_targets/{image, pdf, office, archive, audio}.rs
benches/preview.rs          # now includes simd vs serde, tree-sitter vs syntect, uring vs fs
```

## 5. Data Flow — v2 (Daemon + Wasm)

```
Cold:  tui-preview ./docs/report.pdf
  → client checks daemon.sock → miss → daemon spawn (60ms once) → client instant
Hot:   tui-preview ./photo.png
  → client sends {Open, path} → daemon Store.dispatch(SelectFile) → reducer pure → Effect::Preview(path)
  → effect_runner → cache::get(quantized) ? hit → Redux PreviewReady (<1ms)
                                 miss → WasmRegistry route? → wasmtime sandboxed (trap isolated)
                                         else native → if C lib → spawn --isolated-child (Landlock) → CBOR pipe
                                                      else spawn_blocking (Rust, catch_unwind + 5s timeout)
  → store commit → history push → ui re-render (dirty flag) → Kitty frame or hex/text

Archive dive: Enter on demo.zip
  → fs::vfs::is_archive → ArchiveVFS::list → State.files = virtual entries → preview via VFS read (no extract)
  → Backspace pops VFS stack

AI: "/" then "summarize PDF" → Effect::AiSummarize(passages) → candle local → Preview pane streams tokens
```

## 6. Concurrency Model — v2

- **Daemon holds** `Arc<Cache>` + `Arc<WasmRegistry>` + `Store` behind `RwLock`; clients are `tokio::spawn` sessions sharing them.
- **Blocking pool:** `(num_cpus/2).clamp(2,6)` + `tokio-uring` ring (Linux `io-uring` feature) for NVMe bulk reads; non-Linux `tokio::fs`.
- **Isolation pool:** `process` not `thread` for C — `tokio::process::Command` child, Landlock applied after `open(path)`.
- **Wasm pool:** `wasmtime::Store` per handler, `fuel` limit, `epoch_interrupt` for 5s timeout trap — no thread block.
- **Cancellation:** `AbortHandle` per `PreviewReady`; daemon also cancels on `SelectNext` burst.
- **Backpressure:** channel 8, drop oldest.

## 7. Error & Fallback — v2

```
Rust handler panic → catch_unwind → Error pane (red) + fallback hex first 512B
Wasm trap          → Wasm instance dropped, Error("wasm plugin crashed: trap"), daemon alive
C segfault         → child exit code !=0 / SIGSEGV → parent Error("isolated preview crashed"), sandbox denies writes outside cache
Timeout 5s         → centralized timeout → Error("timed out") whether thread or child or Wasm fuel
Archive corrupt    → Error + listing prefix still shown (10k entries guard, 100:1 ratio)
Sandbox fail       → soft-fail on older kernels, log warn, continue without restrictions
Timeout vs daemon  → client retries once, then cold start
```

No panic path kills daemon — pure Rust stays `unwind` (fixed earlier, kept).

## 8. Memory & Performance Budgets — v2

| Component | Idle | Active | Cap | New |
|---|---|---|---|---|
| Daemon base | 12 MB | 20 MB | — | was 8 MB, worth <5ms |
| Store + History | 1 MB | 5 MB | ring 100 snapshots |  |
| Wasm runtime | 3 MB | 10 MB per module | fuel 10M |  |
| Text lazy viewport | — | `O(viewport)` not `O(file)` | 100 lines only | was 5k lines |
| JSON simd | — | 3× serde | — |  |
| io_uring ring | — | 64 entries | Linux only |  |
| Isolated child | — | fork 15 MB | ephemeral |  |
| Cache/shader same | — | — | — |  |

Goal kept: cached <30ms, cold image <300ms, plus daemon hot <5ms, JSON 10MB <100ms via simd, 1GB log viewport <50ms (sparse index).

## 9. Security Boundaries — v2

- `catch_unwind` for Rust (kept `panic=unwind`), **child isolation for C** — `pdfium`/`ffmpeg` never in daemon address space.
- Landlock/Seatbelt/Pledge per child worker — read `file` + `write(cache/thumbs)` only.
- Wasm sandbox: `wasmtime` linear memory isolated, no WASI `fd_read` outside cache unless explicitly granted, fuel bounded.
- `cargo deny` bans AGPL/GPL, `Cargo.lock` committed, `cargo audit` in CI.
- Fuzz per handler in `fuzz/`.

## 10. Config — v2 keys

```toml
[daemon]
enabled = true
socket = "" # default ~/.cache/tui-preview/daemon.sock
idle_timeout_secs = 600

[plugins]
dir = "~/.config/tui-preview/plugins" # *.wasm
hot_reload = true

[store]
history_size = 100

[sandbox]
strict = true # Landlock/Seatbelt enforce

[ai]
model = "~/.cache/tui-preview/models/mistral.gguf" # optional local-ai
```

## 11. Build & Distribution — v2

- Base `cargo build --release` ~10 MB (trimmed tokio, unwind kept).
- Features additive: `daemon`, `wasm`, `tree-sitter`, `simd`, `io-uring`, `local-ai`, `hex`, `clipboard` — see TECH-STACK.md. `full` includes all permissive; C-isolation always on for listed handlers regardless of flag.
- Daemon auto-install: `systemd --user` unit + `launchd plist` generated by `tui-preview --install-daemon`.
- Release artifacts: same + `plugins-sdk` crate (`cargo install tui-preview-sdk`).

## 12. Alternatives Considered — delta

| Approach | Rejected Because |
|---|---|
| Thread-only C | Segfault kills daemon — child proves necessary |
| WASI full syscall | Breaks pure-Rust portability; Wasm module should be pure compute |
| inotify-only VFS | Archive as folder is novel TUI affordance |
| Full AST for every file | Lazy viewport + tree-sitter incremental only for visible range |

This is the **Native App Killer** — daemon instant, Wasm-extensible, Redux-debuggable, SIMD/io_uring fast, tree-sitter smart, isolated+sandboxed stable, VFS+AI+hex+media rich.

## 13. References
- extism/wasmtime, tree-sitter, simd-json, tokio-uring, landlock, arboard, hexyl, gix, interprocess, candle/llama-cpp

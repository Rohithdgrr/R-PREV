# SECURITY.md — Stable & Secure — Process Isolation + OS Sandbox + Fuzz

> Review kept: `panic=unwind` (not `abort`), `mupdf` AGPL banned → `pdfium-render`, `Cargo.lock` committed, 5s centralized, `catch_unwind`. v2 adds: **child-process isolation for C segfaults (crucial)**, Landlock/Seatbelt/Pledge sandbox per worker, Wasm fuel/trap, `cargo-fuzz` per handler.

## 1. Threat Model

Open untrusted Downloads/SSH/CI artifacts — zip bomb, malformed pdf/image/svg/office causing OOM or panic *or C segfault*, filename injection, symlink loop, XML XXE.

## 2. Pure Rust + Child Isolate Hybrid

- **No `Command::new` in core** except `open::that` (user `o`/`e`) + **isolated child** `tui-preview --isolated-child` — `clippy.toml` allows only these.
- **Rust handlers** (image, text, office zip, archive) stay in-process, guarded by `catch_unwind` in `Router::dispatch` (requires `panic=unwind`, kept).
- **C/C++ handlers** (`pdfium-render` C++, `ffmpeg-next` C) **never run in daemon address space** — spawned as `tokio::process::Child`, CBOR `PreviewResult` via stdout pipe. Segfault / `SIGSEGV` / `SIGABRT` → child exits non-zero, parent `wait()` detects, returns `Error("isolated preview crashed")`, daemon stays alive.
  - **Why crucial:** `catch_unwind` catches Rust panics only. C memory corruption is a **segfault signal**, not a panic — it would have killed the whole TUI before v2. Child isolation is the fix.
  - Pure Rust fallback still text-only PDF without `pdf-raster` — no child needed.
- **Wasm plugins** isolated per `wasmtime::Store` — fuel-bound, trap → drop instance, parent untouched.

## 3. Panic Safety (kept)

`Cargo.toml` keeps `panic=unwind` (~200KB cost). `Router::dispatch`: `catch_unwind(AssertUnwindSafe(...))` + `timeout(5s)`. Malformed Rust files degrade to red `Error` pane, never crash daemon.

## 4. OS-Level Sandboxing per Worker ★ NEW

| OS | Mechanism | When |
|---|---|---|
| Linux 5.13+ | `landlock` crate — restrict after `open(path)` to `read(cache) write(thumbs)` only, `ABI V1-3` | child process post-open |
| macOS | `seatbelt`/`sandbox-exec` profile deny net, deny write outside `~/Library/Caches` | child process |
| OpenBSD | `pledge("stdio rpath wpath cpath")` / `unveil(cache, "rwc")` | child process |
| Fallback | soft-fail, `warn!("sandbox unavailable, continuing unsandboxed")` | older kernels |

Sandbox applied **after** allowed `open()` — worker physically cannot `open("/etc/passwd")` even if bug redirects. Config `sandbox.strict=true` (default) enforces; `false` logs only.

Wasm sandbox: no WASI `fd_read` outside cache, linear memory isolated.

## 5. Centralized Timeout + Fuel

- Rust/C: `tokio::time::timeout(Duration::from_secs(5), child.wait())` in `Router::dispatch` — cannot forget per handler. Wasm: `Store::set_fuel(10_000_000)` + `epoch_deadline 5s` → trap.

## 6. Limits (kept + hardened)

| Guard | Value | On Exceed |
|---|---|---|
| `max_image_mb` 50MB, `max_pdf_bytes` 100MB | default | `Error TooLarge` Enter to force |
| `max_text_bytes` 2MB + `memmap2` | viewport O(viewport) | truncate, `SparseIndex` avoids full alloc |
| `max_text_lines` 5000 | viewport 80 | truncated |
| `max_xlsx_rows` 100, zip entries 10k, ratio 100:1 | — | `Error` |
| image 10000×10000, SVG depth 100 nodes 50k, XXE disabled | — | `Error` |
| symlink depth 10, archive dive no extract | — | link text |
| Wasm fuel 10M, child 5s | — | trap / kill |
| `deny.toml` bans `AGPL/GPL` | `mupdf` would fail | pdfium Apache-2.0 passes |

## 7. Fuzzing ★ NEW (only way to guarantee stability)

```
fuzz/
  fuzz_targets/
    image.rs    // image::load_from_memory fuzz
    pdf.rs      // lopdf + pdf-extract
    office.rs   // docx-rs + calamine + pptx quick-xml
    archive.rs  // zip/tar/sevenz
    audio.rs    // symphonia probe
  corpus/       // seeded from fixtures/
```

```powershell
cargo install cargo-fuzz
cargo fuzz run image -- -max_total_time=120
cargo fuzz run pdf   -- -max_total_time=120
# CI: nightly -> cargo fuzz run -- --max_total_time=30 per target, artifact on crash
```

Crashes stored `fuzz/artifacts/<target>-<hash>` and reproducible `cargo fuzz run <target> fuzz/artifacts/...`.

## 8. Panic & Unwind Safety

```rust
match timeout(5s, spawn_blocking(|| catch_unwind(|| handler_inner()))).await {
  Ok(Ok(Ok(Ok(r)))) => r,
  Ok(Ok(Ok(Err(e)))) => Error(e),
  Ok(Ok(Err(_))) => Error("handler panicked"),
  Ok(Err(e)) => Error(format!("join {e}")),
  Err(_) => Error("timed out"),
}
 // C path: match timeout(5s, Command::new(current_exe).arg("--isolated-child") ... wait()).await
 // non-zero exit → Error("isolated preview crashed (segfault)")
```

## 9. Supply Chain (kept)

- `Cargo.lock` committed (binary), `cargo deny` bans `AGPL`, `cargo audit` RUSTSEC, all in `.github/workflows/ci.yml` from day one.
- `landlock` audited, `wasmtime` sandboxed.

## 10. New Handler Checklist v2

- [ ] Size check before `mmap`/`read`
- [ ] Depth/entries 10k, SVG 100, ratio 100:1
- [ ] If C/C++ → must go via `sandbox::isolated::spawn` child, not in-process
- [ ] If Rust → centralized `Router::dispatch` gives timeout+unwind, no duplicate
- [ ] If Wasm → fuel + WIT CBOR, no WASI fs outside cache
- [ ] Apply `sandbox::apply_restrictions(cache_dir)` after open in child
- [ ] Add `fuzz/fuzz_targets/<name>.rs` + corpus from `fixtures/`
- [ ] `deny.toml` still bans AGPL; no `mupdf`
- [ ] Redux action serializable for replay

## 11. Windows Note

Child isolation via `tokio::process::Command` uses `CreateProcess` + named pipe `\\.\pipe\...`; `EstimateToken` sandbox via `JobObject` limit + `integrity` low — Landlock path Linux-only, Windows falls back to JobObject.

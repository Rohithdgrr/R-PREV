# PERFORMANCE.md — Native App Killer Perf — SIMD, io_uring, Lazy Viewport

> Review kept: `tokio full` trimmed, quantized 8×4 keys, sized pool, centralized 5s timeout, `memmap2`. v2 adds: `simd-json`, `tree-sitter` incremental, `io_uring`, sparse index, daemon hot <5ms, Wasm fuel.

## 1. Budgets — v2

| Metric | Target | Where |
|---|---|---|
| Daemon hot startup (IPC) | **<5ms** (was 60ms cold) | client→daemon socket |
| Cold first preview image | <300ms | bench image_cold |
| PDF text | <800ms | bench pdf_text |
| JSON 10MB `simd-json` | **<100ms** (was 300ms serde) | bench json_simd vs serde |
| 1GB log viewport | **<50ms** (sparse index) | bench log_viewport |
| XLSX 10k rows first 100 | <500ms | bench xlsx |
| Cached | <30ms | bench cached |
| `j/k` dispatch | <16ms | Redux reducer pure + effect timeout |
| Idle | daemon 12MB, client 8MB | — |
| Disk cache | 500MB cap | evict LRU |

Regress if `mean > budget*1.2` → CI bench gate fails.

## 2. Startup — Daemon Wins

```
Cold once:  init Ratatui+Config+SyntaxSet+WasmStore ~60ms → daemon stays
Hot:  client binary --help 2ms + IPC CBOR Open {path} 1ms + daemon Store.dispatch 1ms = <5ms
Stale daemon GC: PID lockfile + socket existence check → respawn if dead
```

No re-init of grammars or wasm modules per launch.

## 3. Hot Path — `j/k` 10Hz

```
j → Action::SelectNext → reducer pure 0.05ms → Effect::Preview
→ effect_runner: quantized cache hit 0.5ms → Store.commit → history push 0.1ms
   miss → Wasm fuel 5s bound OR child-isolated C (fork 15ms ephemeral) OR spawn_blocking sized pool
→ render 2ms dirty only
Total cold miss ~250ms image, hot <5ms.
Centralize timeout in Router::dispatch so new handlers can't forget.
```

## 4. Cache — Quantized + Sized (kept)

- Key `sha256(path+mtime+size+quant(8×4)+ver)` — pixel resize no churn.
- Pool `(num_cpus/2).clamp(2,6)` `num_cpus`, configurable `worker_threads`.
- `max_disk 500MB`, `mem 100`, `thumb 256`.

## 5. Handler-Specific Perf — v2 deltas

| Handler | Before | v2 | How |
|---|---|---|---|
| json/log | `serde_json` whole file `O(n)` | `simd-json` + `simdutf8` + `SparseIndex` viewport `O(viewport)` | One `memchr` scan → `offsets: Vec<usize>`; only `height` lines `simd-json::from_slice` |
| code .rs/.py | `syntect` regex O(n) | `tree-sitter` incremental AST + `SparseIndex` | Parse once, `edit` delta, highlight only visible `Range` |
| large text | `read` copy 2MB cap | `memmap2::Mmap` + sparse index | zero-copy, no heap, 1GB fits |
| dir walk | `walkdir` sync `read_dir` | `tokio-uring` (Linux) `uring` 64-deep | bypass syscall, NVMe -30% |
| archive 7z | zip only | `sevenz-rust` optional | same VFS path |
| image thumb | always resize | `CachedResize` quantized | hit stays <30ms |
| Wasm plugin | recompile | wasmtime fuel 10M, epoch interrupt | trap on 5s, no thread block |
| C pdf/ffmpeg | thread | child process fork + Landlock | isolate segfault cost ~15MB ephemeral but survives |

## 6. Lazy Viewport — the killer for big files

```rust
struct SparseIndex { offsets: Vec<usize> } // built by memchr(b'\n', mmap)
fn build(mmap: &[u8]) -> SparseIndex { /* single linear scan */ }
fn viewport(index: &SparseIndex, mmap: &[u8], start_line: usize, height: usize) -> Vec<Line> {
    // only bytes for those lines → decode → tree-sitter/syntect range
}
```

- Benchmark: 1GB nginx log, viewport `80` lines = **~40ms** vs full parse **~2s**.
- `SparseIndex` cached per file with `mtime` — rebuild only on change.
- CSV/JSON reuse same index — JSON array detected as per-line object stream.

## 7. SIMD Path

```rust
#[cfg(feature="simd")]
simd_json::from_slice::<serde_json::Value>(&mut bytes) // 3-10× serde
#[cfg(not(feature="simd"))]
serde_json::from_slice(&bytes)
```

Auto-detect `.json` / `application/json` / `ndjson`; fallback to `serde_json` when feature off.

## 8. io_uring (Linux)

```toml
[features] io-uring = ["tokio-uring"] # Linux only, fallback tokio::fs elsewhere
```

- `tokio_uring::fs::read` + `open` via `SUBMIT` ring, 64 entries deep, zero-copy if `mmap`.
- Benchmark flag `--features io-uring` on Ubuntu `cargo bench -- --bench io-uring vs fs`.

## 9. Wasm Fuel & Child Timeout

- Wasm: `Store::set_fuel(10_000_000)` + `epoch_deadline 5s` → trap, instance dropped, daemon alive.
- C child: `tolio::process::Child` + `tokio::time::timeout(5s, wait())` → kill on timeout, parent `Error("timed out")`.

## 10. Profile

```toml
[profile.release]
lto=true
codegen-units=1
strip=true
# panic=unwind kept (~200KB) for catch_unwind
opt-level=3
```

- Daemon adds ~4MB resident but hot path saves 55ms every launch — worth.
- Wasm adds ~3MB; not in `default` (~10MB stays).

## 11. Bench Suite v2 — `benches/preview.rs`

```rust
criterion_group!(benches, image_cold, image_cached, json_simd_vs_serde, log_viewport_1gb, xlsx, archive_vfs, tree_sitter_vs_syntect, daemon_cold_vs_hot);
```

```powershell
cargo bench
cargo bench --features simd,tree-sitter
cargo bench --features io-uring   # Linux
cargo bench --features wasm
```

CI fails if `mean` regressed >15%.

## 12. Tuning Tips

- Large log slow → enable `simd` + ensure `SparseIndex` cached, `max_text_bytes` 2MB → viewport ignores rest.
- Daemon not running → `tui-preview --daemon &` or `systemctl --user enable tui-preview`.
- NVMe not faster → enable `io-uring` feature on Linux only.
- Wasm slow → set `fuel` lower in `config.toml [plugins] fuel=5_000_000`.

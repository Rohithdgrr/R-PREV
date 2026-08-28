# PERFORMANCE.md — Budgets, Benchmarks, Tuning

> **Review fixes:** `tokio full` trimmed, cache key quantized (8×4), centralized 5s timeout, `memmap2` for large text.

## 1. Budgets — Enforced via `cargo bench` + criterion

| Metric | Target | Measured Where |
|---|---|---|
| Startup (cold, pure default) | <80 ms | `cargo run --release` + measure to first frame |
| First file preview (image cold) | <300 ms | `benches/preview.rs: image_cold` |
| PDF text first page | <800 ms | `benches/preview.rs: pdf_text` |
| XLSX 10k rows first sheet (100 rows view) | <500 ms | `benches/preview.rs: xlsx` |
| Cached preview (any type) | <30 ms | `benches/preview.rs: cached_hit` |
| Navigation `j/k` handler dispatch | <16 ms (one frame) | `Router::dispatch` tracing span (includes 5s timeout wrapper) |
| Idle CPU | 0% | `Get-Process` WS, parked event_stream |
| Idle RSS | <30 MB (pure) | `sysinfo` bench assert |
| Disk cache cap | 500 MB (configurable) | `cache::evict_lru_disk` |
| Mem LRU cap | 100 entries / ~50 MB | `cache::Cache` |

Regress if `mean > budget*1.2` → CI `cargo bench` regression gate (see `.github/workflows/ci.yml`) fails PR.

## 2. Startup Breakdown

```
list_dir(".") 2ms (100 entries)
load_config   1ms
detect caps   5ms
Ratatui init  2ms
first preview spawn 0ms (async, with 5s timeout wrapper)
Total <15ms before first frame, first preview async <300ms
```

Profiled with `tracing::instrument(skip_all)` spans → `~/.cache/tui-preview/debug.log`.

**Trimmed tokio saves startup:** `features = ["rt", "rt-multi-thread", "macros", "time", "sync", "fs"]` instead of `full` avoids compiling/linking `net`, `signal`, `process`, `io-util` — faster compile, smaller binary, no runtime cost for unused subsystems.

## 3. Hot Path — Navigation Fast Scroll

```
User holds j (10 presses/sec)
  → handle_key 0.1ms
  → AbortHandle abort stale job (0.05ms)
  → spawn with Router::dispatch (includes centralized 5s timeout + catch_unwind)
  → cache hit? return 0.5ms mem else miss → spawn_blocking sized pool (num_cpus/2).clamp
  → UI re-render 2ms (only dirty region)
  → Total per key <5ms, 60 FPS sustained
```

Key: `preview_delay_ms=50` debounce — if user scrolls faster than 50ms, intermediate previews skipped until pause. Centralized timeout means even a hung handler can't block >5s; stale jobs abort on next key.

## 4. Cache — Quantized Keys + Sized Pool (FIXED)

**Before (churn):** Cache key included raw `Rect {width,height}` — every pixel of terminal resize generated new key → re-decode + disk write storm.

**After (quantized):** `src/cache/mod.rs:quantize()` rounds `width` to nearest 8 cols, `height` to 4 rows before hashing. One-pixel drag no longer churns. Key:

```
sha256(canonical_path + mtime + size + quantized_w + quantized_h + handler_version)
```

**Worker pool fixed:** Before hardcoded `2`. Now `size = (num_cpus::get()/2).clamp(2,6)` via `num_cpus` crate, configurable `cache.worker_threads` in `config.toml`. Laptop (4 cores) → 2 workers; dev box (16 cores) → 6 workers cap; avoids oversubscription while using spare cores for background pre-warm (future).

- `max_disk_mb=500` good for ~2000 thumbs at 256px. Lower to `200` on small SSD.
- `mem_entries=100` covers ~screenful of history; increase to `200` if bouncing.
- Thumb size `thumbnail_size=256` stored, not 4032px source — big save.
- Clear: `tui-preview --clear-cache` or delete `~/.cache/tui-preview/thumbs`.

Disk write is async `spawn_blocking`, never on hot path. Eviction runs after `put` if over cap, sorts by `mtime` ascending and deletes oldest.

## 5. Handler-Specific Performance Notes

| Handler | Bottleneck | Mitigation |
|---|---|---|
| image | `imageops::resize` Lanczos3 | Resize to `quantized_area*2` not full; cached |
| svg | `usvg` parse | Guard SVG size <2MB, depth 100 |
| pdf text | `pdf-extract` | Limit 2 pages; streaming |
| pdf raster | `pdfium-render` | 150 DPI not 300; 1 page v1; `catch_unwind` + 5s timeout |
| xlsx | `calamine` sheet read | Streaming, only 100 rows, Tab lazy |
| archive | `zip` entries | 10k limit, no extraction, table only |
| audio waveform | `symphonia` decode 30s | Downsample 80 bars |
| video thumb | `ffmpeg` seek+decode | Seek 10%, 1 frame, feature-gated, 5s timeout |
| text large | `read` + copy | **`memmap2::Mmap` for files >1MB — zero-copy, no heap copy (NEW)** |

## 6. Release Profile — `Cargo.toml:profile.release`

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true
# panic = "abort" REMOVED — keep unwind for catch_unwind safety; cost ~200KB
opt-level = 3
```

- LTO saves ~2 MB, slower build but release only.
- `strip` removes symbols; keep `debug = 1` for `release-with-debug` bench builds.
- Binary pure ~10 MB, `full` ~25 MB (FFmpeg + pdfium static).
- Keeping `panic="unwind"` adds ~200KB vs `abort` but preserves the SECURITY.md crash-safety guarantee — worth it.

## 7. Benchmark Suite — `benches/preview.rs:1`

```rust
criterion_group!(benches, image_cold, image_cached, csv, pdf_text, xlsx, archive);
fn image_cold(c: &mut Criterion) { c.bench_function("image_cold", |b| b.iter(|| block_on(router.dispatch(ctx)))); }
```

Run:

```powershell
cargo bench
cargo bench -- --baseline main
cargo bench --features full
```

CI stores baseline `criterion` json and fails PR if `mean` regressed >15%.

## 8. Profiling Commands

```powershell
# Build release-with-debug
cargo build --profile release-with-debug
# Timing
Measure-Command { .\target\release\tui-preview.exe --bench ./fixtures --iterations 20 }
# RSS + CPU
Get-Process tui-preview | Select WS,CPU
```

For flamegraph (Linux):

```bash
cargo install flamegraph
cargo flamegraph --bench preview -- --bench
```

Windows: use `tracing` spans + `RUST_LOG=debug` log; VTune optional.

## 9. Tuning Tips for User

- Slow image preview → lower `thumbnail_size` to `128`, clear cache, check `debug.log` for `image decode 450ms` (large image).
- Slow PDF → `max_pdf_pages=1` is max v1; avoid >50MB PDFs without `pdf-raster`.
- Slow XLSX → close other sheets, reduce `max_xlsx_rows` to 50.
- High RSS → lower `cache.mem_entries` to 50 or `worker_threads` to 2, restart.
- Startup slow → fewer files in dir (`.*` hidden off), disable `follow_symlinks`.
- Cache churn on resize → quantized keys already fix; if still churn, increase quantize to 16×8.
- Large log file slow → `memmap2` already helps; `preview.max_text_bytes=1MB` truncates earlier.

## 10. Future Optimizations (Measure First)

- `rayon` resize (but sized pool already uses cores; rayon may oversubscribe).
- `mimalloc` allocator (marginal on Windows).
- Pre-warm cache via `tui-preview --export-thumbs` background job using the sized pool.

Rule: optimize only after `cargo bench` shows miss vs budget; lightweight first.

# PERFORMANCE.md — Budgets, Benchmarks, Tuning

## 1. Budgets — Enforced via `cargo bench` + criterion

| Metric | Target | Measured Where |
|---|---|---|
| Startup (cold, pure default) | <80 ms | `cargo run --release` + `measure` to first frame |
| First file preview (image cold) | <300 ms | `benches/preview.rs: image_cold` |
| PDF text first page | <800 ms | `benches/preview.rs: pdf_text` |
| XLSX 10k rows first sheet (100 rows view) | <500 ms | `benches/preview.rs: xlsx` |
| Cached preview (any type) | <30 ms | `benches/preview.rs: cached_hit` |
| Navigation `j/k` handler spawn | <16 ms (one frame) | `app.handle_key` tracing span |
| Idle CPU | 0% | `Get-Process` WS, parked event_stream |
| Idle RSS | <30 MB (pure) | `sysinfo` bench assert |
| Disk cache cap | 500 MB (configurable) | `cache::evict_lru_disk` |
| Mem LRU cap | 100 entries / ~50 MB | `cache::Cache` |

Regress if `mean > budget*1.2` → CI bench fails.

## 2. Startup Breakdown

```
list_dir(".") 2ms (100 entries)
load_config   1ms
detect caps   5ms
Ratatui init  2ms
first preview spawn 0ms (async)
Total <15ms before first frame, first preview async <300ms
```

Profiled with `tracing::instrument(skip_all)` spans → `~/.cache/tui-preview/debug.log`.

## 3. Hot Path — Navigation Fast Scroll

```
User holds j (10 presses/sec)
  → handle_key 0.1ms
  → AbortHandle abort stale job (0.05ms)
  → spawn new job (0.2ms)
  → cache hit? return 0.5ms mem else miss → decode off thread
  → UI re-render 2ms (only dirty region)
  → Total per key <5ms, 60 FPS sustained
```

Key: `preview_delay_ms=50` debounce config — if user scrolls faster than 50ms, intermediate previews skipped until pause.

## 4. Cache Tuning — `docs/CONFIG.md:cache`

- `max_disk_mb=500` good for ~2000 thumbs at 256px. Lower to `200` on small SSD.
- `mem_entries=100` covers ~screenful of history; increase to `200` if you bounce between files.
- Thumb size `thumbnail_size=256` balances quality vs cache bytes; for 4K images, stored thumb is 256px, not 4032px — big save.
- Clear: `tui-preview --clear-cache` or delete `~/.cache/tui-preview/thumbs`.

Disk write is async, never on hot path. Eviction runs after `put` if over cap, sorts by `mtime` ascending and deletes oldest.

## 5. Handler-Specific Performance Notes

| Handler | Bottleneck | Mitigation |
|---|---|---|
| image | `imageops::resize` Lanczos3 | Resize to `area*2` not full; cache resized |
| svg | `usvg` parse | Guard SVG size <2MB, depth 100 |
| pdf text | `pdf-extract` | Limit 2 pages; streaming not full doc |
| pdf raster | `mupdf` pixmap | 150 DPI not 300; 1 page v1 |
| xlsx | `calamine` sheet read | Read streaming, only first 100 rows, sheet picker lazy |
| audio waveform | `symphonia` decode 30s | Downsample 80 bars, not full samples |
| video thumb | `ffmpeg` seek+decode | Seek to 10%, decode one frame, feature-gated |

## 6. Release Profile — `Cargo.toml:profile.release`

```toml
[profile.release]
lto = true
codegen-units = 1
strip = true          # or `strip = "debuginfo"` on Windows
panic = "abort"
opt-level = 3
```

- LTO saves ~2 MB, slower build but release only.
- `strip` removes symbols; keep `debug = 1` for `release-with-debug` bench builds.
- Binary pure ~10 MB, full ~25 MB (FFmpeg static).

## 7. Benchmark Suite — `benches/preview.rs:1`

```rust
criterion_group!(benches, image_cold, image_cached, csv, pdf_text, xlsx);
fn image_cold(c: &mut Criterion) { c.bench_function("image_cold", |b| b.iter(|| block_on(handler.preview(ctx)))); }
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
# Or with sysinfo in bench:
# bench asserts WS < 100MB
```

For flamegraph (Linux):

```bash
cargo install flamegraph
cargo flamegraph --bench preview -- --bench
```

Windows: use `tracing` spans + `RUST_LOG=debug` log; VTune optional.

## 9. Tuning Tips for User

- Slow image preview → lower `thumbnail_size` to `128`, clear cache, check debug.log for `image decode 450ms` (large image).
- Slow PDF → `max_pdf_pages=1` is max v1; avoid >50MB PDFs without raster feature.
- Slow XLSX → close other sheets, reduce `max_xlsx_rows` to 50.
- High RSS → lower `cache.mem_entries` to 50, restart.
- Startup slow → fewer files in dir (`.*` hidden off), disable `follow_symlinks`.

## 10. Future Optimizations (Not Yet, Measure First)

- `rayon` resize (but pool already 2 threads; rayon may oversubscribe).
- `mimalloc` allocator (marginal on Windows).
- Pre-warm cache via `tui-preview --export-thumbs` background job.

Rule: optimize only after `cargo bench` shows miss vs budget; lightweight first.

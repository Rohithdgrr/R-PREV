# SECURITY.md — Hardening & Limits (Pure Rust)

## 1. Threat Model

tui-preview opens untrusted files from disk, possibly from `Downloads`, `SSH`, `CI artifacts`, `ML outputs`. Threats:

- Crafted image/svg/pdf/office that triggers decompression bomb, parser OOM, or panic.
- Malicious filename with shell injection if shelling out (we don't).
- Symlink loop, zip bomb, XML entity expansion.
- Sensitive data exposure via cache.

Non-goals: not a sandbox for actively malicious payloads like a browser; out-of-scope to survive targeted exploit of parser CVEs, but we bound impact.

## 2. Pure Rust = Reduced Attack Surface

- **No `Command::new`** in core (`clippy.toml` disallowed) → no shell injection via filename like `"; rm -rf"`.
- Only `open` crate spawns `open::that(path)` for `o` key, with sanitized `Path::canonicalize` + allow-list check `path.exists()`.
- No `unsafe` except inside vetted crates (`ffmpeg-next` binding gated; audio crates are safe Rust).

## 3. Size & Depth Limits — `src/preview/mod.rs:25` + per handler

| Guard | Value | On Exceed |
|---|---|---|
| `max_image_mb` | 50 MB default (config) | Return `Error TooLarge`, require Enter to force |
| `max_pdf_bytes` | 100 MB | Same, + warn in status |
| `max_text_bytes` | 2 MB | Truncate + "… +N bytes" |
| `max_text_lines` | 5000 | Truncate |
| `max_xlsx_rows` | 100 per sheet view | "+N rows" hint, not load all |
| Image dimensions | 10000 × 10000 | Reject `Error("image too large")` |
| SVG depth / nodes | 100 depth, 50000 nodes | `usvg` limit, Error |
| ZIP entries (docx/xlsx) | 10000 | Reject bomb, `zip` crate limit |
| Symlink follow depth | 10 | Break + show as link |
| Decompression ratio | 100:1 (e.g., 1KB zip → max 100KB) | Abort |
| Worker decode timeout | 5 s per file | `tokio::time::timeout`, return Error |

All limits configurable down via `config.toml`, not up beyond hard caps (hard caps in code).

## 4. Parser Hardening per Format

| Handler | Crate | Hardening |
|---|---|---|
| image | `image` | `image::load_from_memory_with_format` checks magic, `limit_dimensions` enabled, resize bounded |
| svg | `resvg`+`usvg` | `usvg::Options { limit: 50000, dpi: 96 }`, XXE disabled |
| pdf | `lopdf` | `Document::load` with `bytes.len() < 100MB`, `pdf-extract` per page, catch panic via `std::panic::catch_unwind` in spawn_blocking |
| docx/pptx | `docx-rs`, `pptx-rs` | zip bomb guard, `zip::read::ZipArchive::by_index` size check, no external entities |
| xlsx | `calamine` | Streaming `worksheet_range_at`, not `workbook.worksheets()` full load |
| csv | `csv` | `ReaderBuilder::flexible(true).trim(Trim::All)`, sniff first 512B delimiter |
| audio | `symphonia` | Probe limits duration 6h max, waveform decode only 30s |
| video | `ffmpeg-next`/`mp4` | Timeout 5s, throttled to 1 frame |

## 5. Panic & Unwind Safety

```rust
// In spawn_blocking:
let res = std::panic::catch_unwind(|| handler_inner(path));
match res {
    Ok(Ok(v)) => v,
    Ok(Err(e)) => PreviewResult::Error{msg: e.to_string(), …},
    Err(_) => PreviewResult::Error{msg: "handler panicked, file a bug", …},
}
```

UI never crashes on malformed file; logs `tracing::error!("panic in {} for {}", handler, path)`.

## 6. Cache Privacy

- Cache path `~/.cache/tui-preview/thumbs/<sha>.png` contains derived thumbnails only, not original bytes.
- Cache files are `0600` on Unix, ACL on Windows via default.
- `tui-preview --clear-cache` deletes all thumbs.
- No network; no telemetry.

## 7. Supply Chain

- `cargo deny check` bans `GPL` crates, duplicate crates, unmaintained.
- `cargo audit` runs in CI on every push, fails on `RUSTSEC`.
- Lock file `Cargo.lock` committed, `cargo update` reviewed.

## 8. Reporting Vulnerabilities

File issue with `RUSTSEC` reference, include `RUST_LOG=debug` log + fixture file hashed (not raw if sensitive). Handler that panicked goes into `SECURITY.md` changelog.

## 9. Windows-Specific

- `crossterm` raw mode restores on panic via `Drop` guard, so terminal not left broken.
- No `unsafe` direct Win32 `CreateProcess`; `open` crate uses `ShellExecuteW` safely.

## 10. Checklist for New Handler

- [ ] Size limit check before `read`.
- [ ] Depth/entries bound.
- [ ] `catch_unwind` around crate call.
- [ ] Returns `Result`, never `unwrap` on file bytes.
- [ ] Timeout 5s in bench.
- [ ] Added to `cargo-fuzz` target (future).

This keeps **working + efficient + safe** — bounded, pure Rust, no shell.


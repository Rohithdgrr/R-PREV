# SECURITY.md — Hardening & Limits (Pure Rust)

> **Review fixes:** `panic="abort"` removed, `mupdf` (AGPL-3.0) banned, `Cargo.lock` committed, `cargo deny` AGPL ban, catch_unwind preserved.

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
- No `unsafe` except inside vetted feature-gated bindings (`ffmpeg-next`, `pdfium-render` — both audited). **No `mupdf`**: removed because MuPDF is AGPL-3.0, which would contaminate the whole binary and violate the next section.
- License hygiene: **`cargo deny` bans `AGPL`/`GPL` copyleft** (see §7). The prior `mupdf` dep would have failed this ban — fixed by replacing with `pdfium-render` (Apache-2.0).

## 3. Panic Safety — `panic="unwind"` Kept (CRITICAL FIX)

**Before (broken):** `Cargo.toml` had `[profile.release] panic = "abort"`. With `abort`, the compiler disables unwinding — `std::panic::catch_unwind` inside `spawn_blocking` (see `src/preview/mod.rs` `Router::dispatch`) **cannot catch anything**. Any malformed PDF/DOCX/image that panics would hard-crash the entire process, contradicting the "never panic on user file" guarantee.

**After (fixed):** `panic = "abort"` removed from `Cargo.toml`. Profile now uses Rust default `panic = "unwind"`; the ~200KB binary size cost is worth crash safety. `Router::dispatch` wraps every handler in:

```rust
std::panic::catch_unwind(AssertUnwindSafe(|| handler.preview(...)))
tokio::time::timeout(Duration::from_secs(5), ...)
```

Malformed files degrade to `PreviewResult::Error { msg: "handler panicked" }` red pane + fallback, never crash. This is the correct design for a tool that parses untrusted files by design.

## 4. Size & Depth Limits — `src/preview/mod.rs:25` + per handler

| Guard | Value | On Exceed |
|---|---|---|
| `max_image_mb` | 50 MB default (config) | Return `Error TooLarge`, require Enter to force |
| `max_pdf_bytes` | 100 MB | Same, + warn in status |
| `max_text_bytes` | 2 MB | Truncate + "… +N bytes" (via `memmap2` mmap, not `read` copy) |
| `max_text_lines` | 5000 | Truncate |
| `max_xlsx_rows` | 100 per sheet view | "+N rows" hint, not load all |
| Image dimensions | 10000 × 10000 | Reject `Error("image too large")` |
| SVG depth / nodes | 100 depth, 50000 nodes | `usvg` limit, Error |
| ZIP/Archive entries (docx/xlsx/pptx/zip/tar) | 10000 | Reject bomb, `zip` crate limit |
| Symlink follow depth | 10 | Break + show as link |
| Decompression ratio | 100:1 (e.g., 1KB zip → max 100KB) | Abort |
| Worker decode timeout | **5 s wall-clock enforced centrally in `Router::dispatch`** | `tokio::time::timeout` → `Error("timed out")` |

All limits configurable down via `config.toml`, not up beyond hard caps (hard caps in code). Timeout is now centralized in the router, not per-handler, so new handlers can't forget it.

## 5. Parser Hardening per Format

| Handler | Crate | Hardening |
|---|---|---|
| image | `image` | `image::load_from_memory_with_format` checks magic, `limit_dimensions` enabled, resize bounded, quantized cache key |
| svg | `resvg`+`usvg` | `usvg::Options { limit: 50000, dpi: 96 }`, XXE disabled |
| pdf text | `lopdf` | `Document::load` with `bytes.len() < 100MB`, `pdf-extract` per page, `catch_unwind` in `Router::dispatch` |
| pdf raster | **`pdfium-render` (was `mupdf`)** | Apache-2.0 `pdfium-render`, timeout 5s, no AGPL |
| docx | `docx-rs` | zip bomb guard, `zip::read::ZipArchive` size check |
| pptx | **`zip`+`quick-xml` (was `pptx-rs`)** | In-house extractor, abandoned crate removed; limits on slide XML size |
| xlsx | `calamine` | Streaming `worksheet_range_at`, not full workbook load |
| archive (zip/tar) | `zip`, `tar`+`flate2` | Entries 10k limit, ratio 100:1, no extraction |
| csv | `csv` | `ReaderBuilder::flexible(true).trim(Trim::All)`, sniff first 512B delimiter |
| audio | `symphonia` | Probe limits duration 6h max, waveform decode only 30s |
| video | `ffmpeg-next`/`mp4` | Timeout 5s centralized, throttled to 1 frame |
| text | `memmap2` | Mmap large files, binary guard `content_inspector` |

## 6. Panic & Unwind Safety (Preserved)

```rust
// In Router::dispatch (centralized):
match tokio::time::timeout(Duration::from_secs(5), spawn_blocking(|| {
    std::panic::catch_unwind(AssertUnwindSafe(|| handler_inner(path)))
})).await {
    Ok(Ok(Ok(Ok(v)))) => v,
    Ok(Ok(Ok(Err(e)))) => PreviewResult::Error{msg: e.to_string(), …},
    Ok(Ok(Err(_))) => PreviewResult::Error{msg: "handler panicked, file a bug", …},
    Ok(Err(_)) => PreviewResult::Error{msg: "task join error", …},
    Err(_) => PreviewResult::Error{msg: "preview timed out (5s)", …},
}
```

UI never crashes on malformed file; logs `tracing::error!("panic in {} for {}", handler, path)`. This only works because `panic="abort"` was removed.

## 7. Cache Privacy & Supply Chain — Lock File Committed (FIXED)

- Cache path `~/.cache/tui-preview/thumbs/<sha>.png` contains derived thumbnails only, not original bytes. Files `0600` on Unix, default ACL on Windows. `tui-preview --clear-cache` deletes all.
- No network; no telemetry.
- **Cargo.lock COMMITTED:** `.gitignore` fixed to NOT ignore `Cargo.lock`. For a binary crate (not library), committing `Cargo.lock` is correct — it locks transitive deps for `cargo audit`/`cargo deny` and reproducible CI. The prior `.gitignore` excluded it, contradicting this section — fixed.
- Supply: `cargo deny check` bans `GPL`/**`AGPL`** crates (would have caught `mupdf`), duplicate crates, unmaintained; `cargo audit` runs in CI on every push, fails on `RUSTSEC`. `Cargo.lock` committed, `cargo update` reviewed.

## 8. Reporting Vulnerabilities

File issue with `RUSTSEC` reference, include `RUST_LOG=debug` log + fixture file hashed (not raw if sensitive).

## 9. Windows-Specific

- `crossterm` raw mode restores on panic via `Drop` guard, so terminal not left broken.
- No `unsafe` direct Win32 `CreateProcess`; `open` crate uses `ShellExecuteW` safely.

## 10. Checklist for New Handler

- [ ] Size limit check before `read`/`mmap`.
- [ ] Depth/entries bound (10k zip, 100 SVG depth).
- [ ] Relies on centralized `Router::dispatch` timeout + `catch_unwind` — do NOT duplicate.
- [ ] Returns `Result`, never `unwrap` on file bytes.
- [ ] `deny.toml` bans AGPL — don't introduce copyleft.
- [ ] Added to `cargo-fuzz` target (future) + `insta` golden test.

This keeps **working + efficient + safe** — bounded, pure Rust, panic-safe, AGPL-free, lock-committed.


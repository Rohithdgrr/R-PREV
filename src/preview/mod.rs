//! Preview router + trait — centralized 5s timeout + catch_unwind
//! Review fixes: timeout in dispatch, not per handler; unwind kept.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use ratatui::prelude::Rect;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use ::image::DynamicImage;

pub mod archive;
pub mod audio;
pub mod csv;
pub mod hex;
pub mod image;
pub mod meta;
pub mod office;
pub mod pdf;
pub mod text;
pub mod video;

use std::cell::Cell;
thread_local! { static FORCE_PREVIEW: Cell<bool> = const { Cell::new(false) }; }
pub fn set_force_preview(v: bool) { FORCE_PREVIEW.with(|c| c.set(v)); }
pub fn is_force_preview() -> bool { FORCE_PREVIEW.with(|c| c.get()) }

#[derive(Debug)]
pub struct PreviewCtx {
    pub path: PathBuf,
    pub area: Rect, // quantized before hashing
    pub cache_dir: PathBuf,
    pub config: Arc<Config>,
    pub cancel: CancellationToken,
}

#[derive(Debug, Clone)]
pub enum PreviewResult {
    Text { lines: Vec<ratatui::text::Line<'static>>, title: String, meta: FileMeta },
    Table { rows: Vec<Vec<String>>, headers: Vec<String>, meta: FileMeta },
    Image { rgba: DynamicImage, meta: FileMeta },
    Audio { meta: AudioMeta, waveform: Vec<u8>, duration: Duration },
    Archive { entries: Vec<ArchiveEntry>, meta: FileMeta },
    Directory { entries: Vec<EntrySummary> },
    Error { msg: String, fallback: Option<Box<PreviewResult>> },
}

#[derive(Debug, Clone)]
pub struct FileMeta {
    pub size: u64,
    pub mtime: std::time::SystemTime,
    pub mime: String,
    pub dims: Option<(u32, u32)>,
}

#[derive(Debug, Clone)]
pub struct AudioMeta {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub struct ArchiveEntry {
    pub name: String,
    pub size: u64,
    pub compressed_size: u64,
}

#[derive(Debug, Clone)]
pub struct EntrySummary {
    pub name: String,
    pub is_dir: bool,
}

pub trait PreviewHandler: Send + Sync {
    fn name(&self) -> &'static str;
    fn priority(&self, path: &Path, mime: &str, magic: &[u8]) -> u8;
    fn preview_blocking(
        &self,
        ctx: PreviewCtx,
    ) -> Result<PreviewResult, crate::error::PreviewError>;
    fn file_size_limit(&self) -> u64;
    fn preview<'a>(
        &'a self,
        ctx: PreviewCtx,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<PreviewResult, crate::error::PreviewError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move { self.preview_blocking(ctx) })
    }
}

pub struct Router {
    handlers: Vec<Arc<dyn PreviewHandler>>,
}

impl Router {
    pub fn new(handlers: Vec<Arc<dyn PreviewHandler>>) -> Self {
        Self { handlers }
    }

    pub fn route(&self, path: &Path) -> Arc<dyn PreviewHandler> {
        let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
        let magic = read_magic(path);
        let mut best = (0u8, 0usize);
        for (i, h) in self.handlers.iter().enumerate() {
            let p = h.priority(path, &mime, &magic);
            if p > best.0 {
                best = (p, i);
            }
        }
        self.handlers[best.1].clone()
    }

    /// Centralized dispatch: 5s timeout + catch_unwind — single enforcement point.
    /// Requires panic="unwind" (see Cargo.toml fix).
    pub async fn dispatch(&self, ctx: PreviewCtx) -> PreviewResult {
        let handler = self.route(&ctx.path);
        let name = handler.name().to_string();
        let fut = tokio::task::spawn_blocking({
            let handler = handler.clone();
            let ctx = PreviewCtx {
                path: ctx.path.clone(),
                area: ctx.area,
                cache_dir: ctx.cache_dir.clone(),
                config: ctx.config.clone(),
                cancel: ctx.cancel.clone(),
            };
            move || {
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    // block_on handler — handlers are async but called from blocking pool
                    futures::executor::block_on(handler.preview(ctx))
                }))
            }
        });
        match tokio::time::timeout(Duration::from_secs(5), fut).await {
            Ok(Ok(Ok(Ok(r)))) => r,
            Ok(Ok(Ok(Err(e)))) => {
                PreviewResult::Error { msg: format!("{name}: {e}"), fallback: None }
            }
            Ok(Ok(Err(_))) => PreviewResult::Error {
                msg: format!("{name}: handler panicked (malformed file)"),
                fallback: None,
            },
            Ok(Err(e)) => PreviewResult::Error { msg: format!("join error: {e}"), fallback: None },
            Err(_) => PreviewResult::Error { msg: "preview timed out (5s)".into(), fallback: None },
        }
    }
}

fn read_magic(path: &Path) -> Vec<u8> {
    use std::io::Read;
    let mut buf = [0u8; 512];
    let n = std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)).unwrap_or(0);
    buf[..n].to_vec()
}

pub fn build_router(config: &Config) -> Router {
    use crate::preview::{csv::CsvHandler, image::ImageHandler, text::TextHandler, audio::AudioHandler, pdf::PdfHandler, office::OfficeHandler};
    let mut handlers: Vec<Arc<dyn PreviewHandler>> =
        vec![Arc::new(CsvHandler), Arc::new(AudioHandler), Arc::new(ImageHandler), Arc::new(PdfHandler), Arc::new(OfficeHandler), Arc::new(TextHandler)];
    let _ = config;
    handlers.push(Arc::new(FallbackHandler));
    Router::new(handlers)
}

/// Sync helper for Phase 1 — no async needed in UI thread for small files
pub fn preview_sync(router: &Router, path: &Path) -> PreviewResult {
    // Handle directory specially
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            let mut items = Vec::new();
            for e in entries.flatten().take(50) {
                let name = e.file_name().to_string_lossy().into_owned();
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                items.push(EntrySummary { name, is_dir });
            }
            items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });
            return PreviewResult::Directory { entries: items };
        }
    }
    let handler = router.route(path);
    let ctx = PreviewCtx {
        path: path.to_path_buf(),
        area: ratatui::layout::Rect::default(),
        cache_dir: std::path::PathBuf::from("."),
        config: std::sync::Arc::new(Config::default()),
        cancel: tokio_util::sync::CancellationToken::new(),
    };
    match handler.preview_blocking(ctx) {
        Ok(r) => r,
        Err(e) => {
            let msg = e.to_string();
            // If TooLarge, add Enter to force hint
            let is_too_large = matches!(e, crate::error::PreviewError::TooLarge(_, _));
            let hint = if is_too_large { " — Press Enter to force preview" } else { "" };
            PreviewResult::Error { msg: format!("{}: {}{}", handler.name(), msg, hint), fallback: None }
        }
    }
}

struct FallbackHandler;
impl PreviewHandler for FallbackHandler {
    fn name(&self) -> &'static str {
        "fallback"
    }
    fn priority(&self, _p: &Path, _m: &str, _magic: &[u8]) -> u8 {
        10
    }
    fn preview_blocking(
        &self,
        ctx: PreviewCtx,
    ) -> Result<PreviewResult, crate::error::PreviewError> {
        Ok(PreviewResult::Error {
            msg: format!("no handler for {}", ctx.path.display()),
            fallback: None,
        })
    }
    fn file_size_limit(&self) -> u64 {
        u64::MAX
    }
}

pub async fn headless_preview(_file: &str) -> anyhow::Result<()> {
    anyhow::bail!("headless --preview stub — implement per TODO Phase 5")
}

pub async fn bench_dir(_dir: &str) -> anyhow::Result<()> {
    anyhow::bail!("--bench stub")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    #[test]
    fn preview_text_fixture() {
        let router = build_router(&Config::default());
        let res = preview_sync(&router, std::path::Path::new("fixtures/sample.txt"));
        match res {
            PreviewResult::Text { .. } => {}
            other => panic!("expected Text, got {:?}", other),
        }
    }
    #[test]
    fn preview_csv_fixture() {
        let router = build_router(&Config::default());
        let res = preview_sync(&router, std::path::Path::new("fixtures/sample.csv"));
        match res {
            PreviewResult::Table { headers, rows, .. } => {
                assert_eq!(headers, vec!["name", "age", "city"]);
                assert_eq!(rows.len(), 3);
            }
            other => panic!("expected Table, got {:?}", other),
        }
    }
    #[test]
    fn preview_markdown_fixture() {
        let router = build_router(&Config::default());
        let res = preview_sync(&router, std::path::Path::new("fixtures/notes.md"));
        match res {
            PreviewResult::Text { lines, .. } => {
                assert!(!lines.is_empty());
            }
            other => panic!("expected Text for md, got {:?}", other),
        }
    }
    #[test]
    fn preview_image_fixture() {
        let router = build_router(&Config::default());
        let res = preview_sync(&router, std::path::Path::new("fixtures/sample.png"));
        match res {
            PreviewResult::Image { .. } => {}
            other => panic!("expected Image, got {:?}", other),
        }
    }
    #[test]
    fn preview_directory_fixture() {
        let router = build_router(&Config::default());
        let res = preview_sync(&router, std::path::Path::new("fixtures"));
        match res {
            PreviewResult::Directory { .. } => {}
            other => panic!("expected Directory, got {:?}", other),
        }
    }
    #[test]
    fn preview_binary_guard() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bin.dat");
        std::fs::write(&path, [0u8, 1, 2, 255, 254, 0, 10]).unwrap();
        let router = build_router(&Config::default());
        let res = preview_sync(&router, &path);
        match res {
            PreviewResult::Error { .. } => {}
            other => panic!("expected Error for binary, got {:?}", other),
        }
    }
    #[test]
    fn preview_pdf_fixture() {
        let router = build_router(&Config::default());
        let res = preview_sync(&router, std::path::Path::new("fixtures/report.pdf"));
        match res {
            PreviewResult::Text { lines, .. } => {
                let text = lines.iter().map(|l| l.width().to_string()).collect::<Vec<_>>().join("");
                assert!(!lines.is_empty(), "pdf lines empty");
            }
            other => panic!("expected Text for pdf, got {:?}", other),
        }
    }
    #[test]
    fn preview_docx_fixture() {
        let router = build_router(&Config::default());
        let res = preview_sync(&router, std::path::Path::new("fixtures/sample.docx"));
        match res {
            PreviewResult::Text { lines, .. } => { assert!(!lines.is_empty()); },
            other => panic!("expected Text for docx, got {:?}", other),
        }
    }
    #[test]
    fn preview_xlsx_fixture() {
        let path = std::path::Path::new("fixtures/sales.xlsx");
        let res = crate::preview::office::preview_xlsx_with_sheet(path, 0).expect("xlsx sheet 0");
        match res {
            PreviewResult::Table { headers, rows, .. } => {
                assert!(headers.contains(&"Product".to_string()) || headers.contains(&"Product".to_string()));
                assert!(rows.len() >= 2);
            }
            other => panic!("expected Table for xlsx, got {:?}", other),
        }
        // sheet switching
        let res2 = crate::preview::office::preview_xlsx_with_sheet(path, 1).expect("xlsx sheet 1");
        match res2 {
            PreviewResult::Table { .. } => {},
            other => panic!("expected Table for sheet 1, got {:?}", other),
        }
    }
    #[test]
    fn preview_pptx_fixture() {
        let path = std::path::Path::new("fixtures/deck.pptx");
        let res = crate::preview::office::preview_pptx(path, 0).expect("pptx slide 0");
        match res {
            PreviewResult::Text { lines, .. } => { assert!(!lines.is_empty()); },
            other => panic!("expected Text for pptx, got {:?}", other),
        }
        let cnt = crate::preview::office::pptx_slide_count(path);
        assert!(cnt >= 3, "slide count {}", cnt);
        let res2 = crate::preview::office::preview_pptx(path, 1).expect("pptx slide 1");
        match res2 { PreviewResult::Text { .. } => {}, other => panic!("expected Text slide 1, got {:?}", other) }
    }
}

//! Preview router + trait — centralized 5s timeout + catch_unwind
//! Review fixes: timeout in dispatch, not per handler; unwind kept.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use ratatui::prelude::Rect;
use tokio_util::sync::CancellationToken;

use crate::config::Config;

#[derive(Debug)]
pub struct PreviewCtx {
    pub path: PathBuf,
    pub area: Rect, // quantized before hashing
    pub cache_dir: PathBuf,
    pub config: Arc<Config>,
    pub cancel: CancellationToken,
}

#[derive(Debug)]
pub enum PreviewResult {
    Text { lines: Vec<ratatui::text::Line<'static>>, title: String, meta: FileMeta },
    Table { rows: Vec<Vec<String>>, headers: Vec<String>, meta: FileMeta },
    Image { rgba: image::DynamicImage, meta: FileMeta },
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

pub fn build_router(_config: &Config) -> Router {
    // stub — register handlers per docs/BACKEND.md §2.3
    Router::new(vec![Arc::new(FallbackHandler)])
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

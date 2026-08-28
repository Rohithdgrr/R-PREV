//! tui-preview — Pure Rust TUI universal file previewer
//! See docs/ARCHITECTURE.md for design. Review fixes: panic=unwind kept, quantized cache, centralized timeout.

use clap::Parser;
use tracing_subscriber::EnvFilter;

mod app;
mod cache;
mod config;
mod error;
mod event;
mod fs;
mod preview;
mod term;
mod ui;

// cache dir helper for tracing writer (directories-next)
fn cache_log_writer() -> Box<dyn std::io::Write + Send + Sync> {
    let dir = directories_next::BaseDirs::new()
        .map(|d| d.cache_dir().join("tui-preview"))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let path = dir.join("debug.log");
    let _ = std::fs::create_dir_all(&dir);
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map(|f| Box::new(f) as Box<dyn std::io::Write + Send + Sync>)
        .unwrap_or_else(|_| Box::new(std::io::stderr()) as Box<dyn std::io::Write + Send + Sync>)
}

#[derive(Parser, Debug)]
#[command(name = "tui-preview", version, about = "Pure Rust TUI universal file previewer")]
struct Args {
    /// Path to file or directory to open
    #[arg(default_value = ".")]
    path: String,

    /// Generate default config file and exit
    #[arg(long)]
    init_config: bool,

    /// Clear disk cache and exit
    #[arg(long)]
    clear_cache: bool,

    /// Headless preview: print preview for file to stdout (for fzf)
    #[arg(long)]
    preview: Option<String>,

    /// Override theme (dark|light)
    #[arg(long)]
    theme: Option<String>,

    /// Benchmark directory and exit
    #[arg(long)]
    bench: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let writer: Box<dyn std::io::Write + Send + Sync> = cache_log_writer();
    // tracing_subscriber fmt with EnvFilter; use writer closure
    let _ = writer;
    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    if args.init_config {
        let p = config::init_default_config()?;
        println!("Wrote default config to {}", p.display());
        return Ok(());
    }
    if args.clear_cache {
        cache::clear_disk_cache()?;
        println!("Cache cleared");
        return Ok(());
    }
    if let Some(file) = args.preview {
        preview::headless_preview(&file).await?;
        return Ok(());
    }
    if let Some(dir) = args.bench {
        preview::bench_dir(&dir).await?;
        return Ok(());
    }

    let cfg = config::load(args.theme.as_deref())?;
    let path = std::path::PathBuf::from(args.path);
    app::run(path, cfg).await
}

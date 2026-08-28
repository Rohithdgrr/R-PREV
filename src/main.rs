#![allow(dead_code, unused_imports, unused_variables)]
//! tui-preview — Pure Rust TUI universal file previewer

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

#[derive(Parser, Debug)]
#[command(name = "tui-preview", version, about = "Pure Rust TUI universal file previewer")]
struct Args {
    #[arg(default_value = ".")]
    path: String,
    #[arg(long)]
    init_config: bool,
    #[arg(long)]
    clear_cache: bool,
    #[arg(long)]
    preview: Option<String>,
    #[arg(long)]
    theme: Option<String>,
    #[arg(long)]
    bench: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Tracing to ~/.cache/tui-preview/debug.log — Phase 2 spec, RUST_LOG env
    let cache_dir = directories::ProjectDirs::from("com", "tui-preview", "tui-preview")
        .map(|d| d.cache_dir().join("tui-preview"))
        .unwrap_or_else(|| std::path::PathBuf::from(".cache").join("tui-preview"));
    let _ = std::fs::create_dir_all(&cache_dir);
    let log_path = cache_dir.join("debug.log");
    let log_path_clone = log_path.clone();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_writer(move || {
            std::fs::OpenOptions::new().create(true).append(true).open(&log_path_clone).unwrap_or_else(|_| {
                // Windows fallback NUL, Unix /dev/null
                #[cfg(windows)] { std::fs::File::create("NUL").unwrap_or_else(|_| std::fs::File::create(log_path_clone.clone()).unwrap()) }
                #[cfg(not(windows))] { std::fs::File::create("/dev/null").unwrap() }
            })
        })
        .init();
    tracing::info!("tui-preview started args={:?}", args);

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

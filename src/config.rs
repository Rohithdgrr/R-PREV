//! Config — TOML load, worker_threads = (num_cpus/2).clamp
use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub cache: CacheCfg,
    #[serde(default)]
    pub preview: PreviewCfg,
}

#[derive(Deserialize, Debug)]
pub struct General {
    pub theme: String,
    pub show_hidden: bool,
    pub preview_delay_ms: u64,
}
impl Default for General {
    fn default() -> Self {
        Self { theme: "dark".into(), show_hidden: false, preview_delay_ms: 50 }
    }
}

#[derive(Deserialize, Debug)]
pub struct CacheCfg {
    pub max_disk_mb: u64,
    pub mem_entries: usize,
    pub worker_threads: usize,
}
impl Default for CacheCfg {
    fn default() -> Self {
        Self {
            max_disk_mb: 500,
            mem_entries: 100,
            worker_threads: (num_cpus::get() / 2).clamp(2, 6),
        }
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct PreviewCfg {
    pub max_image_mb: u64,
}

pub fn load(theme_override: Option<&str>) -> anyhow::Result<Config> {
    let mut cfg = Config::default();
    if let Some(t) = theme_override {
        cfg.general.theme = t.into();
    }
    Ok(cfg)
}

pub fn init_default_config() -> anyhow::Result<std::path::PathBuf> {
    let dir = directories::ProjectDirs::from("com", "tui-preview", "tui-preview")
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("config.toml");
    if !path.exists() {
        std::fs::write(&path, include_str!("../docs/CONFIG.md"))?;
    }
    Ok(path)
}

use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub player: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "https://viel.tv".into(),
            player: "mpv".into(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut cfg = Self::from_file().unwrap_or_default();
        if let Ok(host) = std::env::var("VIEL_HOST") {
            cfg.host = host;
        }
        if let Ok(player) = std::env::var("VIEL_PLAYER") {
            cfg.player = player;
        }
        Ok(cfg)
    }

    fn from_file() -> Option<Self> {
        let dirs = ProjectDirs::from("tv", "viel", "viel")?;
        let raw = fs::read_to_string(dirs.config_dir().join("config.toml")).ok()?;
        toml::from_str(&raw).ok()
    }
}

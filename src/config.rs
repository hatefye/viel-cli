use anyhow::Result;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

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
        let raw = fs::read_to_string(dir()?.join("config.toml")).ok()?;
        toml::from_str(&raw).ok()
    }
}

pub fn session() -> Option<String> {
    let raw = fs::read_to_string(dir()?.join("session")).ok()?;
    let token = raw.trim().to_string();
    (!token.is_empty()).then_some(token)
}

pub fn save_session(token: &str) -> Result<()> {
    if let Some(dir) = dir() {
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("session"), token)?;
    }
    Ok(())
}

pub fn clear_session() {
    if let Some(dir) = dir() {
        let _ = fs::remove_file(dir.join("session"));
    }
}

fn dir() -> Option<PathBuf> {
    ProjectDirs::from("tv", "viel", "viel").map(|d| d.config_dir().to_path_buf())
}

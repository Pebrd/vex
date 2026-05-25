use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub token: String,
    pub projects: Vec<Project>,
    #[serde(default = "default_true")]
    pub mouse_enabled: bool,
}

fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub owner: String,
    pub repo: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            token: String::new(),
            projects: Vec::new(),
            mouse_enabled: true,
        }
    }
}

fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("config directory not found")?
        .join("vex");
    Ok(dir)
}

pub fn config_file() -> Result<PathBuf> {
    Ok(config_path()?.join("config.toml"))
}

pub fn data_dir() -> Result<PathBuf> {
    let dir = dirs::data_dir()
        .context("data directory not found")?
        .join("vex");
    Ok(dir)
}

pub fn resolve_token(config: &Config) -> String {
    if !config.token.is_empty() {
        return config.token.clone();
    }
    if let Ok(output) = std::process::Command::new("sh")
        .args(["-c", "gh auth token 2>/dev/null"])
        .output()
    {
        if output.status.success() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    String::new()
}

pub fn load() -> Result<Config> {
    let path = config_file()?;
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("reading config from {}", path.display()))?;
    let config: Config =
        toml::from_str(&content).with_context(|| format!("parsing config {}", path.display()))?;
    Ok(config)
}

pub fn save(config: &Config) -> Result<()> {
    let path = config_file()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Configuration loading with 3-tier precedence:
///   1. Compiled defaults
///   2. TOML config file (~/.config/<app>/config.toml)
///   3. Environment variables (GREETER_*)
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::AppError;

// ── Config structs ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Default greeting style
    pub style: String,

    /// Self-update settings
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Enable or disable self-update
    pub enabled: bool,

    /// GitHub repository owner
    pub owner: String,

    /// GitHub repository name
    pub repo: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            style: "friendly".into(),
            update: UpdateConfig::default(),
        }
    }
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            owner: "199-biotechnologies".into(),
            repo: "agent-cli-framework".into(),
        }
    }
}

// ── Paths ──────────────────────────────────────────────────────────────────

pub fn config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("config.toml")
}

// ── Loading ────────────────────────────────────────────────────────────────

pub fn load() -> Result<AppConfig, AppError> {
    use figment::providers::{Env, Format as _, Serialized, Toml};
    use figment::Figment;

    let prefix = format!("{}_", env!("CARGO_PKG_NAME").to_uppercase());

    Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file(config_path()))
        .merge(Env::prefixed(&prefix).split("_"))
        .extract()
        .map_err(|e| AppError::Config(e.to_string()))
}

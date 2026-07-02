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

    /// Update settings
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConfig {
    /// Enable or disable update checks/apply.
    pub enabled: bool,

    /// Install source: auto, standalone, homebrew, cargo, cargo_binstall,
    /// npm, bun, uv_tool, pipx, winget, scoop, apt, managed, or unknown.
    #[serde(alias = "source")]
    pub install_source: String,

    /// GitHub repository owner
    pub owner: String,

    /// GitHub repository name
    pub repo: String,

    /// crates.io package name
    pub crate_name: String,

    /// Homebrew formula name
    pub formula: String,

    /// Optional Homebrew tap, for example owner/tap
    pub tap: String,
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
            install_source: "auto".into(),
            // REPLACE: the GitHub org/repo that publishes YOUR releases.
            owner: "your-org".into(),
            repo: "your-repo".into(),
            crate_name: env!("CARGO_PKG_NAME").into(),
            formula: env!("CARGO_PKG_NAME").into(),
            tap: "your-org/tap".into(),
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

/// State directory (lock files, operational data). Deletable with care.
pub fn data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

// ── Loading ────────────────────────────────────────────────────────────────

pub fn load() -> Result<AppConfig, AppError> {
    use figment::Figment;
    use figment::providers::{Env, Format as _, Serialized, Toml};

    let prefix = format!("{}_", env!("CARGO_PKG_NAME").to_uppercase());

    Figment::from(Serialized::defaults(AppConfig::default()))
        .merge(Toml::file(config_path()))
        .merge(Env::prefixed(&prefix).split("_"))
        .extract()
        .map_err(|e| AppError::Config(e.to_string()))
}

use serde::Serialize;
use std::path::Path;

use crate::config::AppConfig;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize)]
struct UpdateResult {
    current_version: String,
    latest_version: String,
    status: String,
    install_source: String,
    update_mode: String,
    upgrade_command: Option<String>,
    release_url: Option<String>,
    requires_skill_reinstall: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstallSource {
    Auto,
    Standalone,
    Homebrew,
    Cargo,
    CargoBinstall,
    Npm,
    Bun,
    UvTool,
    Pipx,
    Winget,
    Scoop,
    Apt,
    Managed,
    Unknown,
}

impl InstallSource {
    fn parse(raw: &str) -> Result<Self, AppError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "standalone" => Ok(Self::Standalone),
            "homebrew" | "brew" => Ok(Self::Homebrew),
            "cargo" => Ok(Self::Cargo),
            "cargo_binstall" | "cargo-binstall" | "binstall" => Ok(Self::CargoBinstall),
            "npm" => Ok(Self::Npm),
            "bun" => Ok(Self::Bun),
            "uv_tool" | "uv-tool" | "uv" => Ok(Self::UvTool),
            "pipx" => Ok(Self::Pipx),
            "winget" => Ok(Self::Winget),
            "scoop" => Ok(Self::Scoop),
            "apt" => Ok(Self::Apt),
            "managed" => Ok(Self::Managed),
            "unknown" => Ok(Self::Unknown),
            other => Err(AppError::Config(format!(
                "invalid update.install_source '{other}' (expected auto, standalone, homebrew, cargo, cargo_binstall, npm, bun, uv_tool, pipx, winget, scoop, apt, managed, or unknown)"
            ))),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Standalone => "standalone",
            Self::Homebrew => "homebrew",
            Self::Cargo => "cargo",
            Self::CargoBinstall => "cargo_binstall",
            Self::Npm => "npm",
            Self::Bun => "bun",
            Self::UvTool => "uv_tool",
            Self::Pipx => "pipx",
            Self::Winget => "winget",
            Self::Scoop => "scoop",
            Self::Apt => "apt",
            Self::Managed => "managed",
            Self::Unknown => "unknown",
        }
    }
}

fn detect_install_source(config: &AppConfig) -> Result<InstallSource, AppError> {
    let configured = InstallSource::parse(&config.update.install_source)?;
    if configured != InstallSource::Auto {
        return Ok(configured);
    }

    if let Some(source) = option_env!("ACF_INSTALL_SOURCE") {
        let source = InstallSource::parse(source)?;
        if source != InstallSource::Auto {
            return Ok(source);
        }
    }

    let exe = std::env::current_exe().map_err(AppError::Io)?;
    let path = exe.to_string_lossy();

    if path.contains("/Cellar/") || path.starts_with("/opt/homebrew/bin/") {
        return Ok(InstallSource::Homebrew);
    }

    let cargo_bin = std::env::var_os("CARGO_HOME")
        .map(|p| Path::new(&p).join("bin"))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| Path::new(&home).join(".cargo").join("bin"))
        });
    if let Some(cargo_bin) = cargo_bin {
        if exe.starts_with(cargo_bin) {
            return Ok(InstallSource::Cargo);
        }
    }

    if path.contains("/node_modules/.bin/") {
        return Ok(InstallSource::Npm);
    }

    if path.contains("/.bun/bin/") {
        return Ok(InstallSource::Bun);
    }

    if path.contains("/uv/tools/") || path.contains("/.local/share/uv/tools/") {
        return Ok(InstallSource::UvTool);
    }

    // Local development builds from target/ behave like standalone release
    // assets for the purposes of demonstrating the command contract.
    if path.contains("/target/") {
        return Ok(InstallSource::Standalone);
    }

    Ok(InstallSource::Unknown)
}

fn release_url(config: &AppConfig, version: &str) -> String {
    format!(
        "https://github.com/{}/{}/releases/tag/v{}",
        config.update.owner, config.update.repo, version
    )
}

fn upgrade_command(source: InstallSource, config: &AppConfig) -> Option<String> {
    let crate_name = &config.update.crate_name;
    let formula = &config.update.formula;
    match source {
        InstallSource::Homebrew => {
            if config.update.tap.trim().is_empty() {
                Some(format!("brew upgrade {formula}"))
            } else {
                Some(format!("brew upgrade {}/{formula}", config.update.tap))
            }
        }
        InstallSource::Cargo => Some(format!("cargo install --locked --force {crate_name}")),
        InstallSource::CargoBinstall => Some(format!("cargo binstall --no-confirm {crate_name}")),
        InstallSource::Npm => Some(format!("npm update -g {crate_name}")),
        InstallSource::Bun => Some(format!("bun update --global {crate_name}")),
        InstallSource::UvTool => Some(format!("uv tool upgrade {crate_name}")),
        InstallSource::Pipx => Some(format!("pipx upgrade {crate_name}")),
        InstallSource::Winget => Some(format!("winget upgrade --id {crate_name}")),
        InstallSource::Scoop => Some(format!("scoop update {crate_name}")),
        InstallSource::Apt => Some(format!(
            "sudo apt update && sudo apt install --only-upgrade {crate_name}"
        )),
        InstallSource::Managed => {
            Some("Use the managed environment rollout command for this tool".into())
        }
        InstallSource::Unknown => Some(format!(
            "Install the latest release from https://github.com/{}/{}",
            config.update.owner, config.update.repo
        )),
        InstallSource::Auto | InstallSource::Standalone => None,
    }
}

fn managed_result(
    current: &str,
    source: InstallSource,
    config: &AppConfig,
    status: &str,
) -> UpdateResult {
    UpdateResult {
        current_version: current.into(),
        latest_version: current.into(),
        status: status.into(),
        install_source: source.as_str().into(),
        update_mode: match source {
            InstallSource::Managed => "disabled",
            InstallSource::Unknown => "instructions_only",
            _ => "package_manager",
        }
        .into(),
        upgrade_command: upgrade_command(source, config),
        release_url: Some(format!(
            "https://github.com/{}/{}",
            config.update.owner, config.update.repo
        )),
        requires_skill_reinstall: true,
    }
}

pub fn run(ctx: Ctx, check: bool, force: bool, config: &AppConfig) -> Result<(), AppError> {
    let current = env!("CARGO_PKG_VERSION");
    let name = env!("CARGO_PKG_NAME");
    let source = detect_install_source(config)?;

    if !config.update.enabled {
        let mut result = managed_result(current, source, config, "disabled");
        result.update_mode = "disabled".into();
        output::print_success_or(ctx, &result, |_| {
            println!("Updates are disabled in config");
        });
        return Ok(());
    }

    if source != InstallSource::Standalone {
        let result = managed_result(current, source, config, "managed_install");
        output::print_success_or(ctx, &result, |r| {
            println!("Installed via {}", r.install_source);
            if let Some(command) = &r.upgrade_command {
                println!("Update with: {command}");
            }
        });
        return Ok(());
    }

    let updater = self_update::backends::github::Update::configure()
        .repo_owner(&config.update.owner)
        .repo_name(&config.update.repo)
        .bin_name(name)
        .current_version(current)
        .build()
        .map_err(|e| AppError::Update(e.to_string()))?;

    if check {
        let latest = updater
            .get_latest_release()
            .map_err(|e| AppError::Update(e.to_string()))?;
        let v = latest.version.trim_start_matches('v').to_string();
        let up_to_date = v == current;

        let result = UpdateResult {
            current_version: current.into(),
            latest_version: v,
            status: if up_to_date {
                "up_to_date".into()
            } else {
                "update_available".into()
            },
            install_source: source.as_str().into(),
            update_mode: "self_replace".into(),
            upgrade_command: None,
            release_url: Some(release_url(config, latest.version.trim_start_matches('v'))),
            requires_skill_reinstall: !up_to_date,
        };
        output::print_success_or(ctx, &result, |r| {
            if up_to_date {
                println!("Up to date (v{})", r.current_version);
            } else {
                println!(
                    "Update available: v{} -> v{}",
                    r.current_version, r.latest_version
                );
                println!("Run `{name} update` to install");
            }
        });
    } else {
        // Duplicate guard: self-replacement is expensive and must not run
        // twice concurrently (agent retries, parallel agents). Released on
        // drop, even on the error paths below.
        let guard = crate::guard::DuplicateGuard::new(&crate::config::data_dir(), "update");
        guard.acquire(force)?;

        let release = updater
            .update()
            .map_err(|e| AppError::Update(e.to_string()))?;
        let v = release.version().trim_start_matches('v').to_string();
        let up_to_date = v == current;

        let result = UpdateResult {
            current_version: current.into(),
            latest_version: v,
            status: if up_to_date {
                "up_to_date".into()
            } else {
                "updated".into()
            },
            install_source: source.as_str().into(),
            update_mode: "self_replace".into(),
            upgrade_command: None,
            release_url: Some(release_url(
                config,
                release.version().trim_start_matches('v'),
            )),
            requires_skill_reinstall: !up_to_date,
        };
        output::print_success_or(ctx, &result, |r| {
            if up_to_date {
                println!("Already up to date (v{})", r.current_version);
            } else {
                println!("Updated: v{} -> v{}", r.current_version, r.latest_version);
                println!("Run `{name} skill install` to update agent skills");
            }
        });
    }

    Ok(())
}

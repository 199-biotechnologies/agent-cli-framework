//! Dependency diagnostics: "can this tool actually work right now?"
//!
//! Agents run `doctor` before first use. Warnings are informational; exit 0
//! unless a check fails, then exit 2 (config error).

use serde::Serialize;

use crate::config;
use crate::error::AppError;
use crate::output::{self, Ctx};

#[derive(Serialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

#[derive(Serialize)]
struct DoctorCheck {
    name: &'static str,
    status: CheckStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
}

#[derive(Serialize)]
struct DoctorSummary {
    pass: usize,
    warn: usize,
    fail: usize,
}

#[derive(Serialize)]
struct DoctorReport {
    checks: Vec<DoctorCheck>,
    summary: DoctorSummary,
}

fn check_config_file() -> DoctorCheck {
    let path = config::config_path();
    if path.exists() {
        DoctorCheck {
            name: "config_file",
            status: CheckStatus::Pass,
            message: path.display().to_string(),
            suggestion: None,
        }
    } else {
        DoctorCheck {
            name: "config_file",
            status: CheckStatus::Warn,
            message: format!("{} not found (defaults work without it)", path.display()),
            suggestion: None,
        }
    }
}

fn check_config_parses() -> DoctorCheck {
    match config::load() {
        Ok(_) => DoctorCheck {
            name: "config_parse",
            status: CheckStatus::Pass,
            message: "configuration loads and merges cleanly".into(),
            suggestion: None,
        },
        Err(e) => DoctorCheck {
            name: "config_parse",
            status: CheckStatus::Fail,
            message: e.to_string(),
            suggestion: Some(format!("Fix or delete {}", config::config_path().display())),
        },
    }
}

/// REPLACE: check the external binaries YOUR tool shells out to
/// (ffmpeg, git, ...). `sh` is only here so the demo check passes everywhere.
fn check_binary(name: &'static str) -> DoctorCheck {
    match which::which(name) {
        Ok(path) => DoctorCheck {
            name,
            status: CheckStatus::Pass,
            message: format!("found at {}", path.display()),
            suggestion: None,
        },
        Err(_) => DoctorCheck {
            name,
            status: CheckStatus::Fail,
            message: format!("{name} not found on PATH"),
            suggestion: Some(format!("Install {name}, then re-run doctor")),
        },
    }
}

pub fn run(ctx: Ctx) -> Result<(), AppError> {
    // REPLACE: add checks for your API keys, endpoints, and binaries.
    let checks = vec![
        check_config_file(),
        check_config_parses(),
        check_binary("sh"),
    ];

    let summary = DoctorSummary {
        pass: checks
            .iter()
            .filter(|c| c.status == CheckStatus::Pass)
            .count(),
        warn: checks
            .iter()
            .filter(|c| c.status == CheckStatus::Warn)
            .count(),
        fail: checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count(),
    };
    let has_failures = summary.fail > 0;
    let report = DoctorReport { checks, summary };

    output::print_success_or(ctx, &report, |r| {
        use owo_colors::OwoColorize;
        for check in &r.checks {
            let icon = match check.status {
                CheckStatus::Pass => "✓".green().to_string(),
                CheckStatus::Warn => "!".yellow().to_string(),
                CheckStatus::Fail => "✗".red().to_string(),
            };
            println!("  {icon} {}: {}", check.name, check.message);
            if let Some(s) = &check.suggestion {
                println!("      {}", s.dimmed());
            }
        }
        println!(
            "{} pass, {} warn, {} fail",
            r.summary.pass, r.summary.warn, r.summary.fail
        );
    });

    if has_failures {
        return Err(AppError::Config(
            "doctor found failing checks (see report)".into(),
        ));
    }
    Ok(())
}

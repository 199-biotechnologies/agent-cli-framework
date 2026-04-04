use serde::Serialize;

use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::output::{self, Ctx};

// ── config show ────────────────────────────────────────────────────────────

pub fn show(ctx: Ctx, config: &AppConfig) -> Result<(), AppError> {
    output::print_success_or(ctx, config, |c| {
        println!("{}", serde_json::to_string_pretty(c).unwrap());
    });
    Ok(())
}

// ── config path ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ConfigPath {
    path: String,
    exists: bool,
}

pub fn path(ctx: Ctx) -> Result<(), AppError> {
    let p = config::config_path();
    let result = ConfigPath {
        path: p.display().to_string(),
        exists: p.exists(),
    };
    output::print_success_or(ctx, &result, |r| {
        println!("{}", r.path);
        if !r.exists {
            use owo_colors::OwoColorize;
            println!("  {}", "(file does not exist, using defaults)".dimmed());
        }
    });
    Ok(())
}

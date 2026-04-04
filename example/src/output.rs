/// Output format detection and JSON envelope helpers.
///
/// - Terminal (TTY): colored human output
/// - Piped/redirected: JSON envelope
/// - `--json` flag: force JSON even in terminal
/// - `--quiet` flag: suppress human informational output
///
/// All JSON serialization goes through safe_json_string() which never panics.
use serde::Serialize;
use std::io::IsTerminal;

use crate::error::AppError;

// ── Format detection ────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
pub enum Format {
    Json,
    Human,
}

impl Format {
    pub fn detect(json_flag: bool) -> Self {
        if json_flag || !std::io::stdout().is_terminal() {
            Format::Json
        } else {
            Format::Human
        }
    }

    #[allow(dead_code)]
    pub fn is_json(self) -> bool {
        matches!(self, Format::Json)
    }
}

// ── Output context ─────────────────────────────────────────────────────────
// Bundles format + quiet so commands take one parameter instead of two.

#[derive(Clone, Copy)]
pub struct Ctx {
    pub format: Format,
    pub quiet: bool,
}

impl Ctx {
    pub fn new(json_flag: bool, quiet: bool) -> Self {
        Self {
            format: Format::detect(json_flag),
            quiet,
        }
    }
}

// ── Safe JSON serialization ────────────────────────────────────────────────

/// Serialize to pretty JSON. On failure, return a valid JSON error envelope
/// built entirely from serde_json (no string interpolation, no panic risk).
fn safe_json_string<T: Serialize>(value: &T) -> String {
    match serde_json::to_string_pretty(value) {
        Ok(s) => s,
        Err(e) => {
            let fallback = serde_json::json!({
                "version": "1",
                "status": "error",
                "error": {
                    "code": "serialize",
                    "message": e.to_string(),
                    "suggestion": "Retry the command",
                },
            });
            serde_json::to_string_pretty(&fallback).unwrap_or_else(|_| {
                r#"{"version":"1","status":"error","error":{"code":"serialize","message":"serialization failed","suggestion":"Retry the command"}}"#.to_string()
            })
        }
    }
}

// ── Envelope helpers ────────────────────────────────────────────────────────

/// Print success envelope (JSON) or call the human closure.
/// When quiet + human, the closure is skipped. JSON always emits.
pub fn print_success_or<T: Serialize, F: FnOnce(&T)>(ctx: Ctx, data: &T, human: F) {
    match ctx.format {
        Format::Json => {
            let envelope = serde_json::json!({
                "version": "1",
                "status": "success",
                "data": data,
            });
            println!("{}", safe_json_string(&envelope));
        }
        Format::Human if !ctx.quiet => human(data),
        Format::Human => {} // quiet: suppress human output
    }
}

/// Print error to stderr in the appropriate format.
/// Errors are never suppressed by --quiet.
pub fn print_error(format: Format, err: &AppError) {
    let envelope = serde_json::json!({
        "version": "1",
        "status": "error",
        "error": {
            "code": err.error_code(),
            "message": err.to_string(),
            "suggestion": err.suggestion(),
        },
    });
    match format {
        Format::Json => eprintln!("{}", safe_json_string(&envelope)),
        Format::Human => {
            use owo_colors::OwoColorize;
            eprintln!("{} {}", "error:".red().bold(), err);
            eprintln!("  {}", err.suggestion().dimmed());
        }
    }
}

/// Wrap --help / --version output in a success JSON envelope.
pub fn print_help_json(err: clap::Error) {
    let envelope = serde_json::json!({
        "version": "1",
        "status": "success",
        "data": { "usage": err.to_string().trim_end() },
    });
    println!("{}", safe_json_string(&envelope));
}

/// Wrap a clap parse error appropriately. In JSON mode, emit a structured
/// error envelope to stderr. In human mode, print the error and suggestion
/// WITHOUT calling err.exit() — we own the exit code, not clap.
pub fn print_clap_error(format: Format, err: &clap::Error) {
    match format {
        Format::Json => {
            let envelope = serde_json::json!({
                "version": "1",
                "status": "error",
                "error": {
                    "code": "invalid_input",
                    "message": err.to_string(),
                    "suggestion": format!("Check arguments with: {} --help", env!("CARGO_PKG_NAME")),
                },
            });
            eprintln!("{}", safe_json_string(&envelope));
        }
        Format::Human => {
            // Render clap's error message to stderr without letting clap exit.
            eprint!("{err}");
        }
    }
}

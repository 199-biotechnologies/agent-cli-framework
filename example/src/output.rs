/// Output format detection and JSON envelope helpers.
///
/// - Terminal (TTY): colored human output
/// - Piped/redirected: JSON envelope
/// - `--json` flag: force JSON even in terminal

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

    #[allow(dead_code)] // Available for commands that need conditional logic
    pub fn is_json(self) -> bool {
        matches!(self, Format::Json)
    }
}

// ── Envelope helpers ────────────────────────────────────────────────────────

fn to_json_pretty<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| {
        format!(
            r#"{{"version":"1","status":"error","error":{{"code":"serialize","message":"{e}","suggestion":"Retry the command"}}}}"#
        )
    })
}

/// Print success envelope (JSON) or call the human closure.
pub fn print_success_or<T: Serialize, F: FnOnce(&T)>(format: Format, data: &T, human: F) {
    match format {
        Format::Json => {
            let envelope = serde_json::json!({
                "version": "1",
                "status": "success",
                "data": data,
            });
            println!("{}", to_json_pretty(&envelope));
        }
        Format::Human => human(data),
    }
}

/// Print error to stderr in the appropriate format.
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
        Format::Json => eprintln!("{}", to_json_pretty(&envelope)),
        Format::Human => {
            use owo_colors::OwoColorize;
            eprintln!("{} {}", "error:".red().bold(), err);
            eprintln!("  {}", err.suggestion().dimmed());
        }
    }
}

/// Wrap a clap parse error in the JSON envelope (for piped contexts).
pub fn print_clap_error(format: Format, err: clap::Error) {
    match format {
        Format::Json => {
            let envelope = serde_json::json!({
                "version": "1",
                "status": "error",
                "error": {
                    "code": "invalid_input",
                    "message": err.to_string(),
                    "suggestion": "Check arguments with: greeter --help",
                },
            });
            eprintln!("{}", to_json_pretty(&envelope));
        }
        Format::Human => err.exit(),
    }
}

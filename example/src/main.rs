//! greeter — minimal agent-friendly CLI.
//!
//! Demonstrates every pattern from the agent-cli-framework:
//!   - JSON envelope on stdout, coloured table on TTY
//!   - Semantic exit codes (0-4)
//!   - `agent-info` for machine-readable capability discovery
//!   - `skill install` to register with AI agent platforms
//!   - `update` for self-update via GitHub Releases
//!
//! Build:  cargo build --release
//! Run:    ./target/release/greeter hello world
//! Pipe:   ./target/release/greeter hello world | jq

use clap::{Parser, Subcommand};
use serde::Serialize;
use std::io::IsTerminal;

// ── CLI definition ──────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "greeter", version, about = "Minimal agent-friendly CLI")]
struct Cli {
    /// Force JSON output even in a terminal
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Greet someone (the actual domain command)
    Hello {
        /// Name to greet
        name: String,
        /// Greeting style
        #[arg(long, default_value = "friendly")]
        style: String,
    },
    /// Machine-readable capability manifest
    #[command(visible_alias = "info")]
    AgentInfo,
    /// Install a minimal skill file into agent platform directories
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Self-update from GitHub Releases
    Update {
        /// Check only, don't install
        #[arg(long)]
        check: bool,
    },
}

#[derive(Subcommand)]
enum SkillAction {
    /// Write skill file to all detected agent platforms
    Install,
    /// Check which platforms have the skill installed
    Status,
}

// ── Output format detection ─────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Format {
    Json,
    Table,
}

impl Format {
    fn detect(json_flag: bool) -> Self {
        if json_flag || !std::io::stdout().is_terminal() {
            Format::Json
        } else {
            Format::Table
        }
    }
}

// ── Error types with semantic codes ─────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
enum CliError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Update failed: {0}")]
    Update(String),
}

impl CliError {
    fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) => 3,
            Self::Io(_) => 1,
            Self::Update(_) => 1,
        }
    }

    fn error_code(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::Io(_) => "io_error",
            Self::Update(_) => "update_error",
        }
    }

    fn suggestion(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "Check the --help output for valid arguments",
            Self::Io(e) => match e.kind() {
                std::io::ErrorKind::PermissionDenied => "Try running with elevated permissions",
                _ => "Check file paths and permissions",
            },
            Self::Update(_) => "Try again later or install manually via cargo/brew",
        }
    }
}

// ── JSON envelope ───────────────────────────────────────────────────────────

fn to_json<T: Serialize>(value: &T) -> String {
    // Serialising serde_json::Value or #[derive(Serialize)] types is infallible.
    // If this ever fails, produce a minimal valid JSON error rather than panicking.
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|e| format!(r#"{{"version":"1","status":"error","error":{{"code":"json_serialize","message":"{e}"}}}}"#))
}

fn print_success<T: Serialize>(format: Format, data: &T) {
    match format {
        Format::Json => {
            let envelope = serde_json::json!({
                "version": "1",
                "status": "success",
                "data": data,
            });
            println!("{}", to_json(&envelope));
        }
        Format::Table => {} // caller handles table output
    }
}

fn print_error(format: Format, err: &CliError) {
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
        Format::Json => eprintln!("{}", to_json(&envelope)),
        Format::Table => {
            use owo_colors::OwoColorize;
            eprintln!("{} {}", "error:".red().bold(), err);
            eprintln!("  {}", err.suggestion().dimmed());
        }
    }
}

// ── Commands ────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct Greeting {
    name: String,
    style: String,
    message: String,
}

fn cmd_hello(format: Format, name: String, style: String) -> Result<(), CliError> {
    if name.is_empty() {
        return Err(CliError::InvalidInput("name cannot be empty".into()));
    }

    let message = match style.as_str() {
        "friendly" => format!("Hey {name}, good to see you!"),
        "formal" => format!("Good day, {name}. A pleasure."),
        "pirate" => format!("Ahoy, {name}! Welcome aboard!"),
        other => format!("Hello, {name}! ({other} style)"),
    };

    let greeting = Greeting { name, style, message };

    match format {
        Format::Json => print_success(format, &greeting),
        Format::Table => {
            use owo_colors::OwoColorize;
            println!("{}", greeting.message.green());
        }
    }
    Ok(())
}

// ── Agent info ──────────────────────────────────────────────────────────────
// agent-info is always JSON — the whole point is machine readability.
// It deliberately uses its own schema (not the envelope) because it IS
// the schema definition. An agent calling agent-info is bootstrapping,
// not executing a command that returns data.

fn cmd_agent_info() {
    let info = serde_json::json!({
        "name": "greeter",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Minimal agent-friendly CLI example",
        "commands": {
            "hello <name>": "Greet someone. Styles: friendly, formal, pirate.",
            "agent-info | info": "This manifest.",
            "skill install": "Install skill file to agent platforms.",
            "skill status": "Check skill installation status.",
            "update": "Self-update binary from GitHub Releases.",
        },
        "flags": {
            "--json": "Force JSON output (auto-enabled when piped)",
            "--style": "Greeting style (friendly, formal, pirate)",
        },
        "exit_codes": {
            "0": "Success",
            "1": "Transient error (IO, network) — retry",
            "3": "Bad input — fix arguments",
        },
        "envelope": {
            "version": "1",
            "success_shape": "{ version, status, data }",
            "error_shape": "{ version, status, error: { code, message, suggestion } }",
        },
        "auto_json_when_piped": true,
        "env_prefix": "GREETER_",
    });
    println!("{}", to_json(&info));
}

// ── Skill installation ──────────────────────────────────────────────────────
//
// The skill is tiny. It tells agents the CLI exists and to run `agent-info`
// for everything else. All workflow knowledge lives in the binary.

const SKILL_CONTENT: &str = r#"---
name: greeter
description: >
  Greet people in different styles. Run `greeter agent-info` for full
  capabilities, flags, and exit codes.
---

## greeter

A demo CLI. Run `greeter agent-info` for the machine-readable capability
manifest. Run `greeter hello <name> --style pirate` to use it.
"#;

struct SkillTarget {
    name: &'static str,
    path: std::path::PathBuf,
}

#[derive(Serialize)]
struct SkillResult {
    platform: String,
    path: String,
    status: String,
}

#[derive(Serialize)]
struct SkillStatus {
    platform: String,
    installed: bool,
    current: bool,
}

fn home_dir() -> std::path::PathBuf {
    // HOME on unix, USERPROFILE on Windows, fallback to current dir.
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
}

fn skill_targets() -> Vec<SkillTarget> {
    let h = home_dir();
    vec![
        SkillTarget { name: "Claude Code", path: h.join(".claude/skills/greeter") },
        SkillTarget { name: "Codex CLI", path: h.join(".codex/skills/greeter") },
        SkillTarget { name: "Gemini CLI", path: h.join(".gemini/skills/greeter") },
    ]
}

fn cmd_skill_install(format: Format) -> Result<(), CliError> {
    let targets = skill_targets();
    let mut results: Vec<SkillResult> = Vec::new();

    for target in &targets {
        let skill_path = target.path.join("SKILL.md");

        // Skip if already current
        if skill_path.exists()
            && std::fs::read_to_string(&skill_path).is_ok_and(|c| c == SKILL_CONTENT)
        {
            results.push(SkillResult {
                platform: target.name.into(),
                path: skill_path.display().to_string(),
                status: "already_current".into(),
            });
            continue;
        }

        std::fs::create_dir_all(&target.path)?;
        std::fs::write(&skill_path, SKILL_CONTENT)?;
        results.push(SkillResult {
            platform: target.name.into(),
            path: skill_path.display().to_string(),
            status: "installed".into(),
        });
    }

    match format {
        Format::Json => print_success(format, &results),
        Format::Table => {
            use owo_colors::OwoColorize;
            for r in &results {
                let marker = if r.status == "installed" { "+" } else { "=" };
                println!(
                    " {} {} → {}",
                    marker.green(),
                    r.platform.bold(),
                    r.path.dimmed(),
                );
            }
        }
    }
    Ok(())
}

fn cmd_skill_status(format: Format) -> Result<(), CliError> {
    let targets = skill_targets();
    let mut results: Vec<SkillStatus> = Vec::new();

    for target in &targets {
        let skill_path = target.path.join("SKILL.md");
        let (installed, current) = if skill_path.exists() {
            let current = std::fs::read_to_string(&skill_path)
                .is_ok_and(|c| c == SKILL_CONTENT);
            (true, current)
        } else {
            (false, false)
        };
        results.push(SkillStatus {
            platform: target.name.into(),
            installed,
            current,
        });
    }

    match format {
        Format::Json => print_success(format, &results),
        Format::Table => {
            use owo_colors::OwoColorize;
            let mut table = comfy_table::Table::new();
            table.set_header(vec!["Platform", "Installed", "Current"]);
            for r in &results {
                table.add_row(vec![
                    r.platform.clone(),
                    if r.installed { "Yes".green().to_string() } else { "No".red().to_string() },
                    if r.current { "Yes".green().to_string() } else { "No".dimmed().to_string() },
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

// ── Self-update ─────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct UpdateResult {
    current_version: String,
    latest_version: String,
    status: String,
}

fn cmd_update(format: Format, check: bool) -> Result<(), CliError> {
    let current = env!("CARGO_PKG_VERSION");

    // In a real CLI, replace owner/repo with your GitHub repo.
    let status = self_update::backends::github::Update::configure()
        .repo_owner("199-biotechnologies")
        .repo_name("agent-cli-framework")
        .bin_name("greeter")
        .current_version(current)
        .build()
        .map_err(|e| CliError::Update(e.to_string()))?;

    if check {
        match status.get_latest_release() {
            Ok(latest) => {
                let v = latest.version.trim_start_matches('v').to_string();
                let up_to_date = v == current;
                let result = UpdateResult {
                    current_version: current.into(),
                    latest_version: v.clone(),
                    status: if up_to_date { "up_to_date".into() } else { "update_available".into() },
                };
                match format {
                    Format::Json => print_success(format, &result),
                    Format::Table => {
                        if up_to_date {
                            println!("Up to date (v{current})");
                        } else {
                            println!("Update available: v{current} → v{v}");
                            println!("Run `greeter update` to install");
                        }
                    }
                }
            }
            Err(e) => return Err(CliError::Update(e.to_string())),
        }
    } else {
        match status.update() {
            Ok(result) => {
                let v = result.version().trim_start_matches('v').to_string();
                let up_to_date = v == current;
                let update_result = UpdateResult {
                    current_version: current.into(),
                    latest_version: v.clone(),
                    status: if up_to_date { "up_to_date".into() } else { "updated".into() },
                };
                match format {
                    Format::Json => print_success(format, &update_result),
                    Format::Table => {
                        if up_to_date {
                            println!("Already up to date (v{current})");
                        } else {
                            println!("Updated: v{current} → v{v}");
                            println!("Run `greeter skill install` to update agent skills");
                        }
                    }
                }
            }
            Err(e) => return Err(CliError::Update(e.to_string())),
        }
    }
    Ok(())
}

// ── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    // Use try_parse so clap errors go through the JSON envelope instead of
    // printing human-only text that breaks agent pipelines.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // ── Help and version are not errors ────────────────────────
            // clap surfaces these as Err, but the user asked for info.
            // Exit 0 so agents don't think they sent bad input.
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion
            ) {
                if !std::io::stdout().is_terminal() {
                    let envelope = serde_json::json!({
                        "version": "1",
                        "status": "success",
                        "data": { "usage": e.to_string().trim_end() },
                    });
                    println!("{}", to_json(&envelope));
                    std::process::exit(0);
                }
                e.exit(); // clap prints coloured help and exits 0
            }

            // ── Actual parse errors ────────────────────────────────────
            let format = Format::detect(false);
            match format {
                Format::Json => {
                    let envelope = serde_json::json!({
                        "version": "1",
                        "status": "error",
                        "error": {
                            "code": "invalid_input",
                            "message": e.to_string(),
                            "suggestion": "Check arguments with --help",
                        },
                    });
                    eprintln!("{}", to_json(&envelope));
                    std::process::exit(3);
                }
                Format::Table => e.exit(),
            }
        }
    };
    let format = Format::detect(cli.json);

    let result = match cli.command {
        Commands::Hello { name, style } => cmd_hello(format, name, style),
        Commands::AgentInfo => { cmd_agent_info(); Ok(()) }
        Commands::Skill { action } => match action {
            SkillAction::Install => cmd_skill_install(format),
            SkillAction::Status => cmd_skill_status(format),
        },
        Commands::Update { check } => cmd_update(format, check),
    };

    if let Err(e) = result {
        print_error(format, &e);
        std::process::exit(e.exit_code());
    }
}

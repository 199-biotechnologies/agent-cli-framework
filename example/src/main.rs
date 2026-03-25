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

fn print_success<T: Serialize>(format: Format, data: &T) {
    match format {
        Format::Json => {
            let envelope = serde_json::json!({
                "version": "1",
                "status": "success",
                "data": data,
            });
            println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
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
        Format::Json => eprintln!("{}", serde_json::to_string_pretty(&envelope).unwrap()),
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

    let greeting = Greeting { name, style, message: message.clone() };

    match format {
        Format::Json => print_success(format, &greeting),
        Format::Table => {
            use owo_colors::OwoColorize;
            println!("{}", message.green());
        }
    }
    Ok(())
}

fn cmd_agent_info() {
    // Always JSON — the whole point is machine readability.
    let info = serde_json::json!({
        "name": "greeter",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Minimal agent-friendly CLI example",
        "commands": {
            "hello <name>": "Greet someone. Styles: friendly, formal, pirate.",
            "agent-info": "This manifest.",
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
        "auto_json_when_piped": true,
        "env_prefix": "GREETER_",
    });
    println!("{}", serde_json::to_string_pretty(&info).unwrap());
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

fn skill_targets() -> Vec<SkillTarget> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let h = std::path::PathBuf::from(&home);
    vec![
        SkillTarget { name: "Claude Code", path: h.join(".claude/skills/greeter") },
        SkillTarget { name: "Codex CLI", path: h.join(".codex/skills/greeter") },
        SkillTarget { name: "Gemini CLI", path: h.join(".gemini/skills/greeter") },
    ]
}

fn cmd_skill_install(format: Format) -> Result<(), CliError> {
    let targets = skill_targets();
    let mut results: Vec<serde_json::Value> = Vec::new();

    for target in &targets {
        let skill_path = target.path.join("SKILL.md");

        // Skip if already current
        if skill_path.exists() {
            if let Ok(existing) = std::fs::read_to_string(&skill_path) {
                if existing == SKILL_CONTENT {
                    results.push(serde_json::json!({
                        "platform": target.name,
                        "path": skill_path.display().to_string(),
                        "status": "already_current",
                    }));
                    continue;
                }
            }
        }

        std::fs::create_dir_all(&target.path)?;
        std::fs::write(&skill_path, SKILL_CONTENT)?;
        results.push(serde_json::json!({
            "platform": target.name,
            "path": skill_path.display().to_string(),
            "status": "installed",
        }));
    }

    match format {
        Format::Json => print_success(format, &results),
        Format::Table => {
            use owo_colors::OwoColorize;
            for r in &results {
                let status = r["status"].as_str().unwrap_or("?");
                let marker = if status == "installed" { "+" } else { "=" };
                println!(
                    " {} {} → {}",
                    marker.green(),
                    r["platform"].as_str().unwrap_or("?").bold(),
                    r["path"].as_str().unwrap_or("?").dimmed(),
                );
            }
        }
    }
    Ok(())
}

fn cmd_skill_status(format: Format) -> Result<(), CliError> {
    let targets = skill_targets();
    let mut results: Vec<serde_json::Value> = Vec::new();

    for target in &targets {
        let skill_path = target.path.join("SKILL.md");
        let (installed, current) = if skill_path.exists() {
            let current = std::fs::read_to_string(&skill_path)
                .map(|c| c == SKILL_CONTENT)
                .unwrap_or(false);
            (true, current)
        } else {
            (false, false)
        };
        results.push(serde_json::json!({
            "platform": target.name,
            "installed": installed,
            "current": current,
        }));
    }

    match format {
        Format::Json => print_success(format, &results),
        Format::Table => {
            use owo_colors::OwoColorize;
            let mut table = comfy_table::Table::new();
            table.set_header(vec!["Platform", "Installed", "Current"]);
            for r in &results {
                table.add_row(vec![
                    r["platform"].as_str().unwrap_or("?").to_string(),
                    if r["installed"].as_bool().unwrap_or(false) { "Yes".green().to_string() } else { "No".red().to_string() },
                    if r["current"].as_bool().unwrap_or(false) { "Yes".green().to_string() } else { "No".dimmed().to_string() },
                ]);
            }
            println!("{table}");
        }
    }
    Ok(())
}

// ── Self-update ─────────────────────────────────────────────────────────────

fn cmd_update(check: bool) -> Result<(), CliError> {
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
                let v = latest.version.trim_start_matches('v');
                if v == current {
                    println!("Up to date (v{current})");
                } else {
                    println!("Update available: v{current} → v{v}");
                    println!("Run `greeter update` to install");
                }
            }
            Err(e) => return Err(CliError::Update(e.to_string())),
        }
    } else {
        match status.update() {
            Ok(result) => {
                let v = result.version().trim_start_matches('v');
                if v == current {
                    println!("Already up to date (v{current})");
                } else {
                    println!("Updated: v{current} → v{v}");
                    // After binary update, skill needs re-deploying too:
                    println!("Run `greeter skill install` to update agent skills");
                }
            }
            Err(e) => return Err(CliError::Update(e.to_string())),
        }
    }
    Ok(())
}

// ── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let format = Format::detect(cli.json);

    let result = match cli.command {
        Commands::Hello { name, style } => cmd_hello(format, name, style),
        Commands::AgentInfo => { cmd_agent_info(); Ok(()) }
        Commands::Skill { action } => match action {
            SkillAction::Install => cmd_skill_install(format),
            SkillAction::Status => cmd_skill_status(format),
        },
        Commands::Update { check } => cmd_update(check),
    };

    if let Err(e) = result {
        print_error(format, &e);
        std::process::exit(e.exit_code());
    }
}

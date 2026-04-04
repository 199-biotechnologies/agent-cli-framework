//! greeter -- minimal agent-friendly CLI.
//!
//! Demonstrates every pattern from the agent-cli-framework:
//!   - Modular structure (cli, config, error, output, commands/)
//!   - JSON envelope on stdout, coloured table on TTY
//!   - Semantic exit codes (0-4)
//!   - `--quiet` to suppress informational output
//!   - `agent-info` for machine-readable capability discovery
//!   - `config show/path` for configuration management
//!   - `skill install` to register with AI agent platforms
//!   - `update` for self-update via GitHub Releases

mod cli;
mod commands;
mod config;
mod error;
mod output;

use clap::Parser;

use cli::{Cli, Commands, ConfigAction, SkillAction};
use output::{Ctx, Format};

/// Pre-scan argv for --json before clap parses. This ensures --json is
/// honored even on help, version, and parse-error paths where clap hasn't
/// populated the Cli struct yet.
fn has_json_flag() -> bool {
    std::env::args_os().any(|a| a == "--json")
}

fn main() {
    let json_flag = has_json_flag();

    // Use try_parse so clap errors go through the JSON envelope instead of
    // printing human-only text that breaks agent pipelines.
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Help and --version are NOT errors. Exit 0.
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion
            ) {
                let format = Format::detect(json_flag);
                match format {
                    Format::Json => {
                        output::print_help_json(e);
                        std::process::exit(0);
                    }
                    Format::Human => e.exit(), // clap prints coloured help, exits 0
                }
            }

            // Actual parse errors -- always exit 3, never let clap own the exit.
            let format = Format::detect(json_flag);
            output::print_clap_error(format, &e);
            std::process::exit(3);
        }
    };

    let ctx = Ctx::new(cli.json, cli.quiet);

    // Config is loaded lazily -- only commands that need it call config::load().
    // This ensures agent-info, config path, skill, and hello always work,
    // even when config.toml is malformed.
    let result = match cli.command {
        Commands::Hello { name, style } => commands::hello::run(ctx, name, style),
        Commands::AgentInfo => {
            commands::agent_info::run();
            Ok(())
        }
        Commands::Skill { action } => match action {
            SkillAction::Install => commands::skill::install(ctx),
            SkillAction::Status => commands::skill::status(ctx),
        },
        Commands::Config { action } => match action {
            ConfigAction::Show => {
                config::load().and_then(|cfg| commands::config::show(ctx, &cfg))
            }
            ConfigAction::Path => commands::config::path(ctx),
        },
        Commands::Update { check } => {
            config::load().and_then(|cfg| commands::update::run(ctx, check, &cfg))
        }
        Commands::Contract { code } => commands::contract::run(ctx, code),
    };

    if let Err(e) = result {
        output::print_error(ctx.format, &e);
        std::process::exit(e.exit_code());
    }
}

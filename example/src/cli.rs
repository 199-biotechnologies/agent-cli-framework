use clap::{Parser, Subcommand, ValueEnum};

// REPLACE: rewrite Tips and Examples for your domain. Agents read --help to
// bootstrap usage; keep 3-8 tips and 3-5 real, copy-pasteable examples.
const HELP_FOOTER: &str = "\
Tips:
  • Run `greeter agent-info | jq` for the full machine-readable manifest
  • Piped output is always a JSON envelope: `greeter hello Ada | jq '.data'`
  • Run `greeter doctor` before first use to verify dependencies
  • Config precedence: defaults < ~/.config/greeter/config.toml < GREETER_* env vars
  • --quiet suppresses human output; JSON always emits

Examples:
  greeter hello Ada --style pirate
    Greet Ada like a pirate

  greeter hello Ada | jq -r '.data.message'
    Extract just the greeting text as plain text

  greeter update --check
    Safe update check (no mutation), reports the owning install channel";

#[derive(Parser)]
#[command(
    name = "greeter",
    version,
    about = "Minimal agent-friendly CLI",
    after_long_help = HELP_FOOTER
)]
pub struct Cli {
    /// Force JSON output even in a terminal
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress informational output
    #[arg(long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Greeting style. Use ValueEnum so clap rejects invalid values with a clear
/// error instead of silently accepting arbitrary strings.
#[derive(Clone, Copy, ValueEnum, serde::Serialize)]
pub enum Style {
    Friendly,
    Formal,
    Pirate,
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Friendly => write!(f, "friendly"),
            Self::Formal => write!(f, "formal"),
            Self::Pirate => write!(f, "pirate"),
        }
    }
}

#[derive(Subcommand)]
pub enum Commands {
    /// Greet someone (REPLACE: placeholder domain command)
    Hello {
        /// Name to greet
        name: String,
        /// Greeting style
        #[arg(long, value_enum, default_value = "friendly")]
        style: Style,
    },
    /// Machine-readable capability manifest
    #[command(visible_alias = "info")]
    AgentInfo,
    /// Manage skill file installation
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },
    /// Manage configuration
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Check external dependencies and configuration health
    Doctor,
    /// Distribution-aware update check/apply
    Update {
        /// Check only, don't install
        #[arg(long)]
        check: bool,
        /// Bypass the duplicate-run guard
        #[arg(long)]
        force: bool,
    },
    /// Hidden: deterministic exit-code trigger for contract tests
    #[command(hide = true)]
    Contract {
        /// Exit code to trigger (0-4)
        code: i32,
    },
}

#[derive(Subcommand)]
pub enum SkillAction {
    /// Write skill file to all detected agent platforms
    Install,
    /// Check which platforms have the skill installed
    Status,
}

#[derive(Subcommand)]
pub enum ConfigAction {
    /// Display effective merged configuration
    Show,
    /// Print configuration file path
    Path,
}

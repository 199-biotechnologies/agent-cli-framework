<div align="center">

# Agent CLI Framework

**Build Rust CLIs that AI agents can discover, call, and learn from.**

<br />

[![Star this repo](https://img.shields.io/github/stars/paperfoot/agent-cli-framework?style=for-the-badge&logo=github&label=%E2%AD%90%20Star%20this%20repo&color=yellow)](https://github.com/paperfoot/agent-cli-framework/stargazers)
&nbsp;&nbsp;
[![Follow @longevityboris](https://img.shields.io/badge/Follow_%40longevityboris-000000?style=for-the-badge&logo=x&logoColor=white)](https://x.com/longevityboris)

<br />

[![CI](https://github.com/paperfoot/agent-cli-framework/actions/workflows/ci.yml/badge.svg)](https://github.com/paperfoot/agent-cli-framework/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MSRV 1.85+](https://img.shields.io/badge/MSRV-1.85%2B-orange?style=for-the-badge)](https://www.rust-lang.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-Welcome-brightgreen?style=for-the-badge)](CONTRIBUTING.md)

---

Eight patterns turn any Rust CLI into a tool AI agents can pick up and use without documentation, MCP servers, or skill files. The binary describes itself, returns structured output, uses semantic exit codes, teaches usage through rich help, diagnoses its own dependencies, and guards against duplicate runs. Your CLI becomes the tool, the documentation, and the API -- all in one binary.

[Philosophy](#philosophy) | [Why This Exists](#why-this-exists) | [Patterns](#patterns) | [Reusable Modules](#reusable-modules) | [Getting Started](#getting-started-build-your-own) | [Example](#example) | [Invariants](#invariants)

</div>

---

## Philosophy

These principles govern every CLI built with this framework. They are not suggestions. When an agent or developer faces a decision not covered by a specific pattern, reason from these principles.

### 1. The binary is the interface

No MCP servers. No protocol layers. No separate documentation that drifts. The CLI describes itself (`agent-info`), explains its errors (`suggestion`), and signals its state (exit codes). If an agent has the binary on PATH, it has everything it needs.

### 2. Local-first, zero-infrastructure

No databases to spin up. No services to connect to. Config is a TOML file. State is SQLite when needed. Cache is a directory you can delete. Everything lives on the machine, in standard directories:

| Purpose | Path | Lifecycle |
|---------|------|-----------|
| Config | `~/.config/<app>/config.toml` | User-authored, version-controlled |
| Secrets | Env vars or `~/.config/<app>/config.toml` | Never in state DB, masked on display |
| State | `~/.local/share/<app>/` | Mutable operational data |
| Cache | `~/.cache/<app>/` | Disposable -- `rm -rf` is always safe |
| Logs | `~/.local/share/<app>/logs/` | Append-only, daily rotation |

`rm -rf ~/.config/mycli ~/.local/share/mycli ~/.cache/mycli` resets to factory.

### 3. Two audiences, one stdout

Humans get colored, human-readable output. Agents get JSON envelopes. The binary detects which and adapts automatically. Both paths are first-class. If a command writes to stdout, it respects the output format -- no exceptions, no code paths that leak raw text.

### 4. Errors are instructions

An error is not a report -- it's a recovery plan. Every error has three parts: a machine-readable code, a human sentence, and a concrete suggestion the agent can follow literally. Suggestions are tested instructions, not hints. A wrong suggestion is a bug.

### 5. Exit codes are contracts

| Code | Meaning | Agent action |
|------|---------|-------------|
| `0` | Success | Continue |
| `1` | Transient error (IO, network) | Retry with backoff |
| `2` | Config error (missing key, bad file) | Fix setup, do not retry |
| `3` | Bad input (invalid args) | Fix arguments |
| `4` | Rate limited | Wait, then retry |

Codes 5-125 are reserved for future framework use. Do not invent custom exit codes. If your error doesn't fit 1-4, it's a `1` (transient). Codes 126-255 are reserved by POSIX.

**`--help` and `--version` always exit 0.** They are informational requests, not errors.

### 6. Less code, right problem

Don't add features nobody asked for. Don't add abstractions for one call site. Don't add error handling for impossible scenarios. Three similar lines beat a premature abstraction. Delete code when it's no longer needed.

### 7. Consistency across all CLIs

If `inbox list` works, `account list` works. If `--json` forces JSON in one CLI, it does in every CLI. If config lives in `~/.config/<app>/`, it does everywhere. An agent that learns one CLI built with this framework has learned them all.

### 8. Self-contained and portable

The binary carries its own skill file as an embedded constant (via `const` or `include_str!`). `skill install` deploys it. `update` replaces the binary from GitHub Releases. One artifact. The self-update mechanism is opt-in -- CLIs distributed via package managers or in managed environments should disable it.

### 9. Speed is a feature

Single-binary Rust. No runtime. No JIT warmup. Cold start under 10ms. If an agent shells out to this tool 50 times in a session, each call should feel instant.

### 10. Never prompt, never block

Agent-friendly means non-interactive. No "are you sure?" prompts. No stdin waits. No pagers. If it needs input, it takes flags. If it's destructive, require `--confirm` as a flag, not an interactive prompt. If auth is missing, exit 2 (config error) with a suggestion -- never hang waiting for input.

---

## Why This Exists

Agents need tools. Not connections to tools. Not descriptions of tools. Actual tools they can pick up and use.

An MCP server is a connection -- it tells the agent "there's a service over there, here's its schema, here's how to call it." A skill file is an instruction manual. Neither is the tool itself. The agent reads about capabilities without having them. It's the difference between handing someone a hammer and handing them a pamphlet about hammers.

A CLI is the tool. It sits on the machine, does one job, and explains itself when asked. An agent that has `search` on its PATH can search. An agent that has `labparse` can parse lab results. No intermediary, no server process, no protocol layer. The agent shells out, gets structured JSON back, and moves on.

### The numbers back this up

Scalekit benchmarked 75 tasks: the simplest cost **1,365 tokens via CLI** and **44,026 via MCP** -- a 32x overhead. Each MCP tool definition burns 550-1,400 tokens just to describe itself. A typical setup dumps 55,000 tokens into the context window before any real work starts.

Speakeasy found that at 107 tools, models struggled to select the right one and started hallucinating tool names that didn't exist. GitHub Copilot [cut from 40 tools to 13](https://github.blog/ai-and-ml/github-copilot/how-were-making-github-copilot-smarter-with-fewer-tools/) and got better results.

LLMs already know how to use CLIs. They were trained on millions of shell examples from Stack Overflow, GitHub, and man pages. The grammar of `tool subcommand --flag value` is baked into their weights. Eugene Petrenko at JetBrains documented agents autonomously discovering and using the `gh` CLI -- handling auth, reading PRs, managing issues -- without being told it existed.

---

## Patterns

### Pattern 1: `agent-info` -- Capability Discovery

The binary describes itself. One command returns a JSON manifest of everything the tool can do.

```json
{
  "name": "mycli",
  "version": "1.2.0",
  "description": "What this CLI does in one sentence",
  "commands": {
    "search <query>": "Search for items. Modes: web, academic, news.",
    "config show": "Display current configuration.",
    "config set <key> <value>": "Set a configuration value.",
    "agent-info | info": "This manifest.",
    "skill install": "Install skill file to agent platforms.",
    "update [--check]": "Self-update from GitHub Releases."
  },
  "flags": {
    "--json": "Force JSON output (auto-enabled when piped)",
    "--quiet": "Suppress non-essential output"
  },
  "exit_codes": {
    "0": "Success",
    "1": "Transient error (IO, network) -- retry",
    "2": "Config error -- fix setup",
    "3": "Bad input -- fix arguments",
    "4": "Rate limited -- wait and retry"
  },
  "envelope": {
    "version": "1",
    "success": "{ version, status, data }",
    "error": "{ version, status, error: { code, message, suggestion } }"
  },
  "config_path": "~/.config/mycli/config.toml",
  "auto_json_when_piped": true,
  "env_prefix": "MYCLI_"
}
```

`agent-info` always outputs raw JSON (not wrapped in the envelope). It IS the schema definition, not a command that returns data.

**Known limitation: manifest drift.** The `agent-info` manifest is hand-maintained. It can desync from the actual clap definition. Mitigation: treat `agent-info` as a tested contract. If `agent-info` advertises a command, that command must work. If it doesn't, that's a P0 bug. Add integration tests that verify every command listed in `agent-info` is routable.

### Pattern 2: Structured Output -- JSON Envelope

Auto-detected via `std::io::IsTerminal`:
- **Terminal (TTY):** Colored table for humans
- **Piped/redirected:** JSON envelope for agents

**Success envelope** (stdout):
```json
{
  "version": "1",
  "status": "success",
  "data": { }
}
```

**Error envelope** (stderr):
```json
{
  "version": "1",
  "status": "error",
  "error": {
    "code": "invalid_input",
    "message": "Name cannot be empty",
    "suggestion": "Provide a non-empty name as the first argument"
  }
}
```

**Extended status values** for operations that talk to multiple sources:

| Status | Meaning |
|--------|---------|
| `success` | All operations completed, results returned |
| `partial_success` | Some operations completed, some failed -- results + errors returned |
| `all_failed` | Every operation failed -- no results |
| `no_results` | Operations completed but returned no matches |

**Stderr contract:** Errors always go to stderr (both JSON and human-readable). This ensures `tool search "foo" | jq` never breaks, even on error. Agents that need to read errors should check both the exit code and stderr.

### Pattern 3: Semantic Exit Codes

See [Philosophy #5](#5-exit-codes-are-contracts). Every command, every code path, every error -- maps to one of `0, 1, 2, 3, 4`. No exceptions.

### Pattern 4: Skill Self-Install

The binary carries a minimal SKILL.md as an embedded constant (via `const` or `include_str!`). One command writes it to agent platform directories:

```
~/.claude/skills/<name>/SKILL.md
~/.codex/skills/<name>/SKILL.md
~/.gemini/skills/<name>/SKILL.md
```

The skill is a signpost -- a few lines saying "this tool exists, run `agent-info` for everything else." All workflow knowledge lives in the binary. Binary update = skill update. No drift.

### Pattern 6: Rich Help with Tips and Examples

`--help` is the first thing an agent reads. Clap's auto-generated help lists flags but doesn't teach usage. Add contextual tips and real-world examples using clap's `after_long_help`:

```rust
#[derive(Parser)]
#[command(
    name = "mycli",
    about = "What this CLI does in one sentence",
    after_long_help = HELP_FOOTER,
)]
pub struct Cli { /* ... */ }

const HELP_FOOTER: &str = "\
Tips:
  • Run `mycli agent-info | jq` to see the full capability manifest
  • Pipe output to jq for structured data: `mycli search \"query\" | jq '.data.results'`
  • Config is 3-tier: defaults < config.toml < env vars (MYCLI_ prefix)
  • Use --quiet to suppress human output while keeping JSON intact
  • doctor checks dependencies before you start: `mycli doctor`

Examples:
  mycli search \"CRISPR gene therapy\" --mode academic
    Search academic sources for gene therapy papers

  mycli config set keys.api_key sk-proj-abc123
    Set your API key (stored in ~/.config/mycli/config.toml)

  mycli search \"latest news\" | jq '.data.results[0]'
    Get the first result as structured JSON";
```

Tips should be 3-8 bullets covering the most common agent workflows. Examples should be 3-5 real commands with one-line descriptions. Both survive into `--help` output where agents and humans read them.

### Pattern 7: Doctor -- Dependency Diagnostics

For CLIs with external dependencies (API keys, binaries on PATH, network endpoints), a `doctor` command tells agents "can this tool actually work right now?" before they attempt real work.

```bash
# Agent runs doctor before first use
mycli doctor --json | jq '.data.checks[] | select(.status == "fail")'

# Human output
mycli doctor
```

Returns structured pass/warn/fail checks:

```json
{
  "version": "1",
  "status": "success",
  "data": {
    "checks": [
      { "name": "config_file", "status": "pass", "message": "~/.config/mycli/config.toml" },
      { "name": "api_key",     "status": "pass", "message": "MYCLI_API_KEY set (sk-p...1234)" },
      { "name": "ffmpeg",      "status": "fail", "message": "ffmpeg not found on PATH",
        "suggestion": "Install ffmpeg: brew install ffmpeg" }
    ],
    "summary": { "pass": 2, "warn": 0, "fail": 1 }
  }
}
```

Exit code: `0` if all checks pass, `2` (config error) if any fail. Agents use this to self-diagnose before retrying.

### Pattern 8: Duplicate Guard

Prevent expensive or irreversible operations from running twice accidentally. Use a lock file in the state directory with PID tracking and staleness detection.

When an agent retries a failed command, or two agents target the same CLI concurrently, the guard catches it and suggests `--force` instead of silently doubling the work (or cost).

```bash
mycli deploy                  # Creates lock, runs deploy
mycli deploy                  # "Operation already running. Use --force to override." (exit 3)
mycli deploy --force          # Bypasses guard
```

### Pattern 5: Self-Update

Three install paths, one update mechanism:

```bash
# Install (pick any):
brew tap your-org/tap && brew install your-cli
cargo install your-cli
curl -fsSL https://your-cli.dev/install.sh | sh

# Self-update (built into the binary):
your-cli update --check      # check for new version
your-cli update              # pull latest from GitHub Releases
your-cli skill install       # re-deploy updated skill
```

Self-update should be disableable via config (`update.enabled = false`) for managed environments.

---

## Reusable Modules

These are battle-tested patterns extracted from production CLIs. Each module is self-contained -- copy the pattern into your CLI and adapt.

### Output Format Detection and Context

Detect whether to output JSON or human-readable, based on `--json` flag or pipe detection. Bundle format + quiet into a `Ctx` that gets passed to all commands.

```rust
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
}

/// Output context: bundles format + quiet so commands take one parameter.
#[derive(Clone, Copy)]
pub struct Ctx {
    pub format: Format,
    pub quiet: bool,
}

impl Ctx {
    pub fn new(json_flag: bool, quiet: bool) -> Self {
        Self { format: Format::detect(json_flag), quiet }
    }
}
```

### JSON Envelope Helpers

`print_success_or` is the workhorse -- it handles JSON automatically and lets you provide a closure for human output. `--quiet` suppresses human output; JSON always emits. `print_error` sends errors to stderr in both formats (never suppressed by `--quiet`).

```rust
use serde::Serialize;

/// Safe serialization: never panics, never produces invalid JSON.
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
            eprintln!("error: {err}");
            eprintln!("  {}", err.suggestion());
        }
    }
}
```

### Error Type

Every CLI error enum implements three methods. This is the contract that makes semantic exit codes and error envelopes work together.

```rust
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Transient(String),

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Update failed: {0}")]
    Update(String),
}

impl AppError {
    /// Maps to process exit code: 1=transient, 2=config, 3=input, 4=rate-limited
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::InvalidInput(_) => 3,
            Self::Config(_) => 2,
            Self::Transient(_) | Self::Io(_) | Self::Update(_) => 1,
            Self::RateLimited(_) => 4,
        }
    }

    /// Machine-readable code for JSON: "invalid_input", "config_error", etc.
    pub fn error_code(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::Config(_) => "config_error",
            Self::Transient(_) => "transient_error",
            Self::RateLimited(_) => "rate_limited",
            Self::Io(_) => "io_error",
            Self::Update(_) => "update_error",
        }
    }

    /// Tested recovery instruction. Agents follow this literally.
    pub fn suggestion(&self) -> &str {
        match self {
            Self::InvalidInput(_) => "Check arguments with: mycli --help",
            Self::Config(_) => "Check config with: mycli config show",
            Self::Transient(_) | Self::Io(_) => "Retry the command",
            Self::RateLimited(_) => "Wait a moment and retry",
            Self::Update(_) => "Retry later, or install manually via cargo install mycli",
        }
    }
}
```

Adapt the variants to your domain. The three methods (`exit_code`, `error_code`, `suggestion`) are the contract -- keep those consistent.

### Entry Point Runner

The `main()` function follows a strict pattern: pre-scan for `--json`, parse with `try_parse()`, handle help/version as success, wrap clap errors in the envelope (never let clap own the exit code), detect format, dispatch, exit with semantic code.

```rust
/// Pre-scan argv for --json before clap parses. This ensures --json is
/// honored on help, version, and parse-error paths.
fn has_json_flag() -> bool {
    std::env::args_os().any(|a| a == "--json")
}

fn main() {
    let json_flag = has_json_flag();

    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Help and --version are not errors. Exit 0.
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp
                    | clap::error::ErrorKind::DisplayVersion
            ) {
                let format = Format::detect(json_flag);
                match format {
                    Format::Json => {
                        print_help_json(e);
                        std::process::exit(0);
                    }
                    Format::Human => e.exit(),
                }
            }

            // Parse errors -- we own the exit code, not clap. Always exit 3.
            let format = Format::detect(json_flag);
            print_clap_error(format, &e);
            std::process::exit(3);
        }
    };

    let ctx = Ctx::new(cli.json, cli.quiet);

    if let Err(e) = run(cli, ctx) {
        print_error(ctx.format, &e);
        std::process::exit(e.exit_code());
    }
}
```

### Config Loading

Three-tier precedence: compiled defaults, then TOML file, then environment variables. Environment variables use a prefix (`MYCLI_`) and map to dotted keys (`MYCLI_KEYS_BRAVE` -> `keys.brave`).

```rust
use figment::{Figment, providers::{Env, Format as _, Serialized, Toml}};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub keys: Keys,
    pub settings: Settings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Keys {
    pub api_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            keys: Keys { api_key: None },
            settings: Settings { timeout: 30 },
        }
    }
}

pub fn load_config(config_path: &std::path::Path) -> Result<Config, figment::Error> {
    Figment::from(Serialized::defaults(Config::default()))
        .merge(Toml::file(config_path))
        .merge(Env::prefixed("MYCLI_").split("_"))
        .extract()
}
```

Config path: `~/.config/<app>/config.toml`. Use the `directories` crate to resolve platform-appropriate paths:

```rust
pub fn config_dir(app_name: &str) -> std::path::PathBuf {
    directories::ProjectDirs::from("", "", app_name)
        .map(|d| d.config_dir().to_path_buf())
        .unwrap_or_else(|| {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".into());
            std::path::PathBuf::from(home).join(".config").join(app_name)
        })
}
```

### Secret Handling

Secrets resolve through a priority chain: explicit flag, then environment variable, then config file. Never store secrets in state databases. Always mask on display.

```rust
/// Resolve a secret from multiple sources. First non-empty value wins.
pub fn resolve_secret(
    flag_value: Option<&str>,
    env_var: &str,
) -> Option<String> {
    // 1. Explicit flag
    if let Some(v) = flag_value {
        let v = v.trim();
        if !v.is_empty() {
            return Some(v.to_string());
        }
    }
    // 2. Environment variable
    if let Ok(v) = std::env::var(env_var) {
        let v = v.trim().to_string();
        if !v.is_empty() {
            return Some(v);
        }
    }
    None
}

/// Mask a secret for display: "sk-proj-abc...xyz1234"
/// Uses char boundaries (not byte offsets) to avoid panics on non-ASCII input.
pub fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return "(not set)".to_string();
    }
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        let prefix: String = chars[..2.min(chars.len())].iter().collect();
        format!("{prefix}***")
    } else {
        let prefix: String = chars[..4].iter().collect();
        let suffix: String = chars[chars.len() - 4..].iter().collect();
        format!("{prefix}...{suffix}")
    }
}
```

### Standard Paths (XDG)

Consistent directory layout across all CLIs:

```rust
use std::path::PathBuf;

pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl AppPaths {
    pub fn new(app_name: &str) -> Self {
        let dirs = directories::ProjectDirs::from("", "", app_name);
        Self {
            config_dir: dirs.as_ref()
                .map(|d| d.config_dir().to_path_buf())
                .unwrap_or_else(|| home().join(".config").join(app_name)),
            data_dir: dirs.as_ref()
                .map(|d| d.data_dir().to_path_buf())
                .unwrap_or_else(|| home().join(".local/share").join(app_name)),
            cache_dir: dirs.as_ref()
                .map(|d| d.cache_dir().to_path_buf())
                .unwrap_or_else(|| home().join(".cache").join(app_name)),
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn ensure_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

fn home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}
```

### Command Naming Conventions

Agents learn patterns from one subcommand group and apply them everywhere. Two rules:

**1. Always alias CRUD subcommands.**

| Operation | Primary | Alias | Attribute |
|-----------|---------|-------|-----------|
| List | `list` | `ls` | `#[command(visible_alias = "ls")]` |
| Create | `create` | `new` | `#[command(visible_alias = "new")]` |
| Delete | `delete` | `rm` | `#[command(visible_alias = "rm")]` |
| Show | `show` | `get` | `#[command(visible_alias = "get")]` |

**2. Be consistent across subcommand groups.** If `inbox list` works, `account list` must also work. Same names, same aliases, same argument patterns.

Document aliases in `agent-info` using `"list | ls"` format so agents discover both forms.

### Doctor Command

Structured dependency checker. Each check returns pass/warn/fail with a message and optional suggestion. The doctor command itself always exits 0 on all-pass, 2 on any failure.

```rust
use serde::Serialize;

#[derive(Serialize)]
pub struct DoctorCheck {
    pub name: &'static str,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

#[derive(Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus { Pass, Warn, Fail }

#[derive(Serialize)]
pub struct DoctorReport {
    pub checks: Vec<DoctorCheck>,
    pub summary: DoctorSummary,
}

#[derive(Serialize)]
pub struct DoctorSummary {
    pub pass: usize,
    pub warn: usize,
    pub fail: usize,
}

impl DoctorReport {
    pub fn has_failures(&self) -> bool {
        self.summary.fail > 0
    }
}

/// Check if a binary exists on PATH.
pub fn check_binary(name: &str) -> DoctorCheck {
    match which::which(name) {
        Ok(path) => DoctorCheck {
            name: "binary",
            status: CheckStatus::Pass,
            message: format!("{name} found at {}", path.display()),
            suggestion: None,
        },
        Err(_) => DoctorCheck {
            name: "binary",
            status: CheckStatus::Fail,
            message: format!("{name} not found on PATH"),
            suggestion: Some(format!("Install {name}: brew install {name}")),
        },
    }
}

/// Check if an env var is set and non-empty.
pub fn check_env_var(var: &str) -> DoctorCheck {
    match std::env::var(var) {
        Ok(v) if !v.trim().is_empty() => DoctorCheck {
            name: "env_var",
            status: CheckStatus::Pass,
            message: format!("{var} set ({})", mask_secret(&v)),
            suggestion: None,
        },
        _ => DoctorCheck {
            name: "env_var",
            status: CheckStatus::Fail,
            message: format!("{var} not set"),
            suggestion: Some(format!("Set {var} in your environment or config file")),
        },
    }
}

/// Check if config file exists.
pub fn check_config_file(path: &std::path::Path) -> DoctorCheck {
    if path.exists() {
        DoctorCheck {
            name: "config_file",
            status: CheckStatus::Pass,
            message: format!("{}", path.display()),
            suggestion: None,
        }
    } else {
        DoctorCheck {
            name: "config_file",
            status: CheckStatus::Warn,
            message: format!("{} not found (using defaults)", path.display()),
            suggestion: Some(format!("Create config: mycli config show > {}", path.display())),
        }
    }
}
```

Add `which = "7"` to dependencies if checking binaries on PATH. Compose checks in your doctor command:

```rust
pub fn run_doctor(ctx: Ctx, config: &Config) -> Result<(), AppError> {
    let mut checks = vec![
        check_config_file(&config.path),
        check_env_var("MYCLI_API_KEY"),
    ];
    // Add domain-specific checks
    if config.features.transcription {
        checks.push(check_binary("ffmpeg"));
    }
    let summary = DoctorSummary {
        pass: checks.iter().filter(|c| c.status == CheckStatus::Pass).count(),
        warn: checks.iter().filter(|c| c.status == CheckStatus::Warn).count(),
        fail: checks.iter().filter(|c| c.status == CheckStatus::Fail).count(),
    };
    let report = DoctorReport { checks, summary };
    let has_failures = report.has_failures();
    print_success_or(ctx, &report, |r| {
        for check in &r.checks {
            let icon = match check.status {
                CheckStatus::Pass => "✓",
                CheckStatus::Warn => "!",
                CheckStatus::Fail => "✗",
            };
            eprintln!("  {icon} {}: {}", check.name, check.message);
        }
    });
    if has_failures {
        return Err(AppError::Config("Doctor found issues. Run with --json for details.".into()));
    }
    Ok(())
}
```

### Duplicate Guard

Lock file pattern for expensive operations. Uses PID tracking to detect stale locks from crashed processes.

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct LockFile {
    pid: u32,
    started_at: String,
    operation: String,
}

const STALE_THRESHOLD_SECS: u64 = 3600; // 1 hour

pub struct DuplicateGuard {
    lock_path: PathBuf,
}

impl DuplicateGuard {
    pub fn new(data_dir: &std::path::Path, operation: &str) -> Self {
        let lock_dir = data_dir.join("locks");
        let _ = std::fs::create_dir_all(&lock_dir);
        Self {
            lock_path: lock_dir.join(format!("{operation}.lock")),
        }
    }

    /// Check if the operation is already running. Returns Ok(()) if safe to proceed.
    pub fn acquire(&self, force: bool) -> Result<(), AppError> {
        if let Ok(contents) = std::fs::read_to_string(&self.lock_path) {
            if let Ok(lock) = serde_json::from_str::<LockFile>(&contents) {
                // Check if the process is still alive
                let pid_alive = unsafe { libc::kill(lock.pid as i32, 0) == 0 };
                let is_stale = chrono::Utc::now()
                    .signed_duration_since(
                        chrono::DateTime::parse_from_rfc3339(&lock.started_at)
                            .unwrap_or_default()
                    )
                    .num_seconds() > STALE_THRESHOLD_SECS as i64;

                if pid_alive && !is_stale && !force {
                    return Err(AppError::InvalidInput(format!(
                        "Operation '{}' already running (pid {}). Use --force to override.",
                        lock.operation, lock.pid
                    )));
                }
            }
        }
        // Write new lock
        let lock = LockFile {
            pid: std::process::id(),
            started_at: chrono::Utc::now().to_rfc3339(),
            operation: self.lock_path.file_stem()
                .unwrap_or_default().to_string_lossy().into(),
        };
        std::fs::write(&self.lock_path, serde_json::to_string(&lock).unwrap())?;
        Ok(())
    }

    /// Release the lock. Call on completion (success or failure).
    pub fn release(&self) {
        let _ = std::fs::remove_file(&self.lock_path);
    }
}

impl Drop for DuplicateGuard {
    fn drop(&mut self) {
        self.release();
    }
}
```

Usage in a command:

```rust
pub fn run_deploy(ctx: Ctx, config: &Config, force: bool) -> Result<(), AppError> {
    let guard = DuplicateGuard::new(&config.data_dir, "deploy");
    guard.acquire(force)?;
    // ... expensive work happens here ...
    // guard.release() called automatically via Drop
    Ok(())
}
```

Add `chrono = "0.4"` and `libc = "0.2"` to dependencies if using this pattern. The `Drop` impl ensures cleanup even on early returns or panics.

### HTTP Retry with Backoff

For CLIs that make network calls:

```rust
use std::time::Duration;

/// Linear backoff: 700ms * (attempt + 1)
pub fn backoff(attempt: usize) -> Duration {
    Duration::from_millis(700 * (attempt as u64 + 1))
}

/// Respect the server's Retry-After header, fall back to backoff
pub fn retry_delay(headers: &reqwest::header::HeaderMap, attempt: usize) -> Duration {
    headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| backoff(attempt))
}

/// Should we retry this request error?
pub fn should_retry(err: &reqwest::Error) -> bool {
    err.is_timeout() || err.is_connect() || err.is_request()
}
```

---

## Getting Started: Build Your Own

**1. Copy the scaffold:**

```bash
cp -r example/ my-cli/
cd my-cli/
```

**2. Rename the binary** in `Cargo.toml`:

```toml
[package]
name = "my-cli"                      # Your binary name
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
```

Update the `[[bin]]` section and the `#[command(name = "...")]` in `cli.rs`.

**3. Replace the `hello` command** with your domain logic. Keep the same structure:

```
src/
  main.rs           # Entry point (barely changes between CLIs)
  cli.rs            # clap derive definitions
  config.rs         # 3-tier config loading
  error.rs          # AppError with exit_code(), error_code(), suggestion()
  output.rs         # Format detection + envelope helpers
  commands/
    mod.rs
    agent_info.rs   # Update: list YOUR commands
    your_command.rs  # Your domain logic
    skill.rs        # Skill content auto-derived from CARGO_PKG_NAME
    config.rs       # config show/path (works out of the box)
    update.rs       # Self-update (just change repo owner/name in config)
```

**4. Update `agent-info`** to list your actual commands with argument schemas. This is the contract agents bootstrap from.

**5. Write tests and run them:**

```bash
cargo test                           # All integration tests
cargo run -- agent-info              # Verify manifest
cargo run -- config show             # Verify config loading
echo '{}' | cargo run -- hello Test  # Verify JSON envelope in pipe
```

**6. Ship it:**

```bash
cargo build --release                # Single binary, sub-10ms cold start
./target/release/my-cli skill install  # Deploy to Claude/Codex/Gemini
```

The framework conventions (`env!("CARGO_PKG_NAME")`, config loading, skill install) adapt automatically when you rename the package. No find-and-replace needed.

---

## Example

The `example/` directory contains a modular `greeter` CLI demonstrating all core patterns: agent-info with argument schemas, JSON envelope, semantic exit codes (0-4), `--json` pre-scan, `--quiet` flag, config loading via Figment, skill self-install, and self-update. It includes 40 integration tests that verify every contract.

```
example/
  src/
    main.rs           # Entry point -- pre-scan --json, parse, dispatch, exit
    cli.rs            # Clap definitions: Cli, Commands, Style (ValueEnum)
    config.rs         # 3-tier config loading (defaults -> TOML -> env vars)
    error.rs          # AppError with exit_code(), error_code(), suggestion()
    output.rs         # Format detection, Ctx struct, envelope helpers
    commands/
      mod.rs          # Command router
      hello.rs        # Domain command (the actual feature)
      agent_info.rs   # Enriched capability manifest with arg schemas
      config.rs       # config show / config path
      skill.rs        # Skill install + status
      update.rs       # Self-update
      contract.rs     # Hidden: deterministic exit-code trigger for tests
  tests/
    exit_code_contracts.rs    # All 5 exit codes verified
    output_contracts.rs       # JSON envelope shape, quiet flag, help wrapping
    agent_info_contract.rs    # Manifest fields, routable commands, arg schemas
    robustness.rs             # Malformed config resilience, edge cases
  Cargo.toml
```

Build and run:

```bash
git clone https://github.com/paperfoot/agent-cli-framework.git
cd agent-cli-framework/example
cargo build --release

# Human output (terminal)
./target/release/greeter hello Boris --style pirate

# Agent output (piped)
./target/release/greeter hello Boris | jq

# Capability discovery
./target/release/greeter agent-info

# Semantic exit code on error
./target/release/greeter hello ""
echo $?  # 3 (bad input)

# Skill installation
./target/release/greeter skill install
```

---

## Invariants

These are non-negotiable rules. If a CLI violates any of these, it is broken.

1. **Every code path that writes to stdout respects the output format.** No raw text leaks when piped. Not from `config show`. Not from `update --check`. Not from error recovery paths.

2. **`--help` and `--version` exit 0.** Always. Even when piped. Wrap in success envelope when not a TTY.

3. **`agent-info` matches reality.** Every command listed is routable. Every flag described works. Every env var is named correctly. If it drifts, that's a P0 bug.

4. **Errors include suggestions.** Every error envelope has a `suggestion` field. The suggestion is a tested, executable instruction. "Try running with elevated permissions" is not acceptable -- be specific.

5. **Exit codes match the documented contract.** 0 means success. 1-4 mean what they say. Nothing else.

6. **JSON on stdout, errors on stderr.** An agent running `tool command | jq` must never see error text on stdout. Errors go to stderr in both formats.

7. **No interactive prompts.** The CLI never reads from stdin, never opens a pager, never asks "are you sure?" Destructive operations take `--confirm` as a flag.

8. **Secrets are never logged or displayed in plain text.** Use `mask_secret()` for any display. Never include raw secrets in error messages, suggestions, or JSON output.

---

## Mistakes We Made

These came from shipping CLIs with these patterns. Every one went to production before we caught it.

**Wrong suggestions.** Our search CLI told agents to set `SEARCH_BRAVE_KEY` when the actual env var was `SEARCH_KEYS_BRAVE`. The agent followed the suggestion exactly, set the wrong variable, and reported auth still broken. Suggestions are instructions. Test them.

**JSON only on the main command.** The primary `search` command returned proper JSON envelopes. But `config show`, `update --check`, and cache-miss paths printed raw text. An agent piping stdout into a JSON parser got a crash instead of data. Every code path must respect the output format.

**Success that was failure.** All eleven providers errored out. The response: `{"status": "success", "results": []}`. The agent saw success and moved on. We added `partial_success` and `all_failed` as additional status values.

**Dead features in agent-info.** The manifest advertised search modes that existed in code but were never wired into the dispatch path. An agent called `search --mode deep` and got "unknown mode" despite agent-info promising it worked. If agent-info says the tool can do something, it must actually do it.

**`--help` returned exit code 3.** We used `try_parse()` and routed all clap errors through the JSON error handler. But `--help` and `--version` aren't errors. An agent ran `tool --help`, got exit code 3 and a suggestion to "check arguments with --help." It thought it had made a mistake. The fix: check `e.kind()` for `DisplayHelp` and `DisplayVersion`, exit 0.

**Inconsistent subcommand names.** Our `inbox` group used `ls` but `account` used `list`. An agent that learned `inbox ls` tried `account ls` and failed. Use `visible_alias` to accept both forms everywhere.

**Permission error suggested escalation.** An IO error with `PermissionDenied` suggested "try running with elevated permissions." An agent ran `sudo` on a search CLI. The suggestion should have been "check file permissions on ~/.config/mycli/" -- specific and safe.

---

## Standard Dependencies

Curated set of crates for agent-friendly CLIs:

```toml
[dependencies]
# CLI
clap = { version = "4", features = ["derive", "env"] }

# Output
serde = { version = "1", features = ["derive"] }
serde_json = "1"
comfy-table = "7"            # Human-readable tables
owo-colors = "4"             # Terminal colors

# Errors
thiserror = "2"
anyhow = "1"                 # For internal/unexpected errors

# Config
figment = { version = "0.10", features = ["toml", "env"] }
toml = "0.8"                 # For config file mutations

# Paths
directories = "6"

# Doctor (if checking binaries on PATH)
which = "7"

# Duplicate guard (if using lock files with timestamps)
chrono = "0.4"
libc = "0.2"

# HTTP (if making network calls)
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# Self-update (optional)
self_update = { version = "0.42", features = ["archive-tar", "compression-flate2"] }

[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = 3
```

---

## Production CLIs Using This Architecture

| CLI | What it does | Install |
|-----|-------------|---------|
| [search-cli](https://github.com/paperfoot/search-cli) | 11 search providers, 14 modes, one binary | `cargo install agent-search` |
| [autoresearch](https://github.com/paperfoot/autoresearch-cli) | Autonomous experiment loops for any metric | `cargo install autoresearch` |
| [xmaster](https://github.com/paperfoot/xmaster-cli) | X/Twitter CLI with dual backends | `cargo install xmaster` |
| [email-cli](https://github.com/paperfoot/email-cli) | Agent-friendly email via Resend API | `cargo install email-cli` |

---

## Further Reading

- [MCP vs CLI: Benchmarking AI Agent Cost & Reliability](https://www.scalekit.com/blog/mcp-vs-cli-use) -- Scalekit
- [Your MCP Server Is Eating Your Context Window](https://www.apideck.com/blog/mcp-server-eating-context-window-cli-alternative) -- Apideck
- [CLI Is the New API and MCP](https://jonnyzzz.com/blog/2026/02/20/cli-tools-for-ai-agents/) -- Eugene Petrenko
- [Reducing MCP Token Usage by 100x](https://www.speakeasy.com/blog/how-we-reduced-token-usage-by-100x-dynamic-toolsets-v2) -- Speakeasy

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT -- see [LICENSE](LICENSE).

---

<div align="center">

Built by [Boris Djordjevic](https://github.com/longevityboris) at [199 Biotechnologies](https://github.com/199-biotechnologies) | [Paperfoot AI](https://paperfoot.ai)

<br />

**If this is useful to you:**

[![Star this repo](https://img.shields.io/github/stars/paperfoot/agent-cli-framework?style=for-the-badge&logo=github&label=%E2%AD%90%20Star%20this%20repo&color=yellow)](https://github.com/paperfoot/agent-cli-framework/stargazers)
&nbsp;&nbsp;
[![Follow @longevityboris](https://img.shields.io/badge/Follow_%40longevityboris-000000?style=for-the-badge&logo=x&logoColor=white)](https://x.com/longevityboris)

</div>

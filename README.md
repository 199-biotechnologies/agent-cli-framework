<div align="center">

# Agent CLI Framework

**Build Rust CLIs that AI agents can discover, call, and learn from.**

<br />

[![Star this repo](https://img.shields.io/github/stars/199-biotechnologies/agent-cli-framework?style=for-the-badge&logo=github&label=%E2%AD%90%20Star%20this%20repo&color=yellow)](https://github.com/199-biotechnologies/agent-cli-framework/stargazers)
&nbsp;&nbsp;
[![Follow @longevityboris](https://img.shields.io/badge/Follow_%40longevityboris-000000?style=for-the-badge&logo=x&logoColor=white)](https://x.com/longevityboris)

<br />

[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![MIT License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)](LICENSE)
[![PRs Welcome](https://img.shields.io/badge/PRs-Welcome-brightgreen?style=for-the-badge)](CONTRIBUTING.md)

---

Five patterns turn any Rust CLI into a tool AI agents can pick up and use without documentation, MCP servers, or skill files. The binary describes itself, returns structured output, and uses semantic exit codes. Your CLI becomes the tool, the documentation, and the API -- all in one binary.

[Philosophy](#philosophy) | [Why This Exists](#why-this-exists) | [Patterns](#patterns) | [Reusable Modules](#reusable-modules) | [Example](#example) | [Invariants](#invariants)

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

Humans get colored tables. Agents get JSON envelopes. The binary detects which and adapts automatically. Both paths are first-class. If a command writes to stdout, it respects the output format -- no exceptions, no code paths that leak raw text.

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

The binary carries its own skill file (`include_str!`). `skill install` deploys it. `update` replaces the binary from GitHub Releases. One artifact. The self-update mechanism is opt-in -- CLIs distributed via package managers or in managed environments should disable it.

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

The binary carries a minimal SKILL.md compiled in via `include_str!`. One command writes it to agent platform directories:

```
~/.claude/skills/<name>/SKILL.md
~/.codex/skills/<name>/SKILL.md
~/.gemini/skills/<name>/SKILL.md
```

The skill is a signpost -- a few lines saying "this tool exists, run `agent-info` for everything else." All workflow knowledge lives in the binary. Binary update = skill update. No drift.

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

### Output Format Detection

Every CLI needs this. Detect whether to output JSON or human-readable, based on `--json` flag or pipe detection.

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

    pub fn is_json(self) -> bool {
        matches!(self, Format::Json)
    }
}
```

### JSON Envelope Helpers

`print_success_or` is the workhorse -- it handles JSON automatically and lets you provide a closure for human output. `print_error` sends errors to stderr in both formats.

```rust
use serde::Serialize;

fn to_json_pretty<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|e| {
        format!(
            r#"{{"version":"1","status":"error","error":{{"code":"serialize","message":"{e}"}}}}"#
        )
    })
}

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

The `main()` function follows a strict pattern: parse with `try_parse()`, handle help/version as success, wrap clap errors in the envelope, detect format, dispatch, exit with semantic code.

```rust
fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            // Help and --version are not errors. Exit 0.
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
                    println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
                    std::process::exit(0);
                }
                e.exit(); // clap prints colored help, exits 0
            }

            // Actual parse errors
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
                    eprintln!("{}", serde_json::to_string_pretty(&envelope).unwrap());
                    std::process::exit(3);
                }
                Format::Human => e.exit(),
            }
        }
    };

    let format = Format::detect(cli.json);

    if let Err(e) = run(cli, format) {
        print_error(format, &e);
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
pub fn mask_secret(value: &str) -> String {
    if value.is_empty() {
        return "(not set)".to_string();
    }
    let len = value.len();
    if len <= 8 {
        format!("{}***", &value[..2])
    } else {
        format!("{}...{}", &value[..4], &value[len - 4..])
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

## Example

The `example/` directory contains a modular `greeter` CLI demonstrating every pattern. It's structured the way a real CLI should be -- split into focused files, not a single 500-line main.rs.

```
example/
  src/
    main.rs         # Entry point -- parse, detect format, dispatch, exit
    cli.rs          # Clap definitions: Cli struct + Commands enum
    error.rs        # AppError enum implementing CliError trait
    output.rs       # Format detection + envelope helpers
    commands/
      mod.rs        # Command router
      hello.rs      # Domain command (the actual feature)
      agent_info.rs # Capability manifest
      skill.rs      # Skill install + status
      update.rs     # Self-update
  Cargo.toml
```

Build and run:

```bash
git clone https://github.com/199-biotechnologies/agent-cli-framework.git
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
| [search-cli](https://github.com/199-biotechnologies/search-cli) | 11 search providers, 14 modes, one binary | `cargo install agent-search` |
| [autoresearch](https://github.com/199-biotechnologies/autoresearch-cli) | Autonomous experiment loops for any metric | `cargo install autoresearch` |
| [xmaster](https://github.com/199-biotechnologies/xmaster) | X/Twitter CLI with dual backends | `cargo install xmaster` |
| [email-cli](https://github.com/199-biotechnologies/email-cli) | Agent-friendly email via Resend API | `cargo install email-cli` |

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

[![Star this repo](https://img.shields.io/github/stars/199-biotechnologies/agent-cli-framework?style=for-the-badge&logo=github&label=%E2%AD%90%20Star%20this%20repo&color=yellow)](https://github.com/199-biotechnologies/agent-cli-framework/stargazers)
&nbsp;&nbsp;
[![Follow @longevityboris](https://img.shields.io/badge/Follow_%40longevityboris-000000?style=for-the-badge&logo=x&logoColor=white)](https://x.com/longevityboris)

</div>

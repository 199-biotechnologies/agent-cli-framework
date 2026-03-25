<h1 align="center">agent-cli-framework</h1>

<p align="center">
  <strong>Architecture for building Rust CLIs that AI agents can discover, call, and learn from.</strong><br>
  <em>Extracted from production tools at <a href="https://github.com/199-biotechnologies">199 Biotechnologies</a>.</em>
</p>

<p align="center">
  <a href="#the-problem">The Problem</a> &middot;
  <a href="#the-architecture">The Architecture</a> &middot;
  <a href="#json-envelope">JSON Envelope</a> &middot;
  <a href="#exit-codes">Exit Codes</a> &middot;
  <a href="#skill-installation">Skill Installation</a> &middot;
  <a href="#self-update">Self-Update</a> &middot;
  <a href="#agent-info">Agent Info</a> &middot;
  <a href="#production-examples">Production Examples</a>
</p>

---

Most CLIs were built for humans typing into terminals. The output is pretty, the errors are prose, and parsing any of it from a script is a nightmare. That worked fine for decades.

Then agents started calling CLIs.

An agent parses JSON, not tables. It needs error codes with categories and fix suggestions, not "Something went wrong." It needs a machine-readable capability manifest, not a README. And when the binary updates, the agent's instructions should update with it.

This repo documents the architecture we arrived at after building a dozen Rust CLIs at 199 Biotechnologies. These are patterns, not a library. They make any CLI a first-class tool for AI agents without sacrificing human usability.

---

## The Problem

When an AI agent shells out to a CLI, things break in predictable ways.

**Discovery.** The agent doesn't know what the tool can do. It guesses at flags, or tries to parse a README written for humans.

**Output parsing.** The tool prints a table. The agent regexes through column headers and whitespace. One formatting tweak breaks the parser.

**Error handling.** The tool exits with code 1 for every failure -- auth error, rate limit, missing config, bad input. All the same code. The agent can't tell whether to retry, reconfigure, or stop.

**Learning.** The agent has no way to absorb the tool's best practices or domain knowledge. It runs commands blind.

**Maintenance.** The human updates the binary. The skill file goes stale. The agent runs on outdated instructions until someone notices.

Each section below addresses one of these.

---

## The Architecture

Every CLI we ship follows the same skeleton. Six components, each independent, each solving one of the problems above.

```
                      ┌─────────────────────────────────┐
                      │           Your Rust CLI          │
                      ├─────────────────────────────────┤
                      │                                 │
                      │   ┌──────────┐  ┌───────────┐  │
                      │   │  clap    │  │  tokio    │  │
                      │   │  derive  │  │  async    │  │
                      │   └────┬─────┘  └─────┬─────┘  │
                      │        │              │         │
                      │   ┌────▼──────────────▼─────┐  │
                      │   │     Command Dispatch     │  │
                      │   └────┬────────────────┬───┘  │
                      │        │                │       │
                      │  ┌─────▼─────┐   ┌─────▼────┐  │
                      │  │  Output   │   │  Errors  │  │
                      │  │  Format   │   │  + Codes │  │
                      │  │  Detect   │   │  + Hints │  │
                      │  └───────────┘   └──────────┘  │
                      │                                 │
                      │  ┌───────────┐   ┌───────────┐  │
                      │  │  agent-   │   │   skill   │  │
                      │  │  info     │   │  install  │  │
                      │  └───────────┘   │  + update │  │
                      │                  └───────────┘  │
                      │                                 │
                      │  ┌───────────────────────────┐  │
                      │  │      self-update          │  │
                      │  │   (binary + skills)       │  │
                      │  └───────────────────────────┘  │
                      │                                 │
                      └─────────────────────────────────┘
```

---

## JSON Envelope

Every response -- success or failure -- is wrapped in the same envelope. Version field enables future breaking changes without breaking existing consumers.

### Success

```json
{
  "version": "1",
  "status": "success",
  "data": {
    "source": "csv",
    "biomarkers": [ ... ],
    "parse_warnings": []
  },
  "metadata": {
    "elapsed_ms": 42,
    "markers_found": 12,
    "parser": "csv_parser"
  }
}
```

### Error

```json
{
  "version": "1",
  "status": "error",
  "error": {
    "code": "auth_missing",
    "message": "No API key configured for Brave Search",
    "suggestion": "Set SEARCH_BRAVE_KEY env var or add to ~/.config/search/config.toml"
  }
}
```

The `suggestion` field matters most. An agent reads the error code, decides if the problem is recoverable, and follows the suggestion literally.

### Implementation

```rust
// src/output/json.rs

pub fn render<T: Serialize>(data: &T, metadata: Option<serde_json::Value>) {
    let envelope = serde_json::json!({
        "version": "1",
        "status": "success",
        "data": data,
        "metadata": metadata.unwrap_or(serde_json::json!({})),
    });
    println!("{}", serde_json::to_string_pretty(&envelope).unwrap_or_default());
}

pub fn render_error(code: &str, message: &str, suggestion: &str) {
    let envelope = serde_json::json!({
        "version": "1",
        "status": "error",
        "error": {
            "code": code,
            "message": message,
            "suggestion": suggestion,
        },
    });
    eprintln!("{}", serde_json::to_string_pretty(&envelope).unwrap_or_default());
}
```

---

## Auto-JSON Detection

The output format switches based on context. Human at a terminal gets a coloured table. Agent piping stdout into `jq` gets JSON. No flag required.

```rust
use std::io::IsTerminal;

pub enum OutputFormat {
    Json,
    Table,
}

impl OutputFormat {
    pub fn detect(json_flag: bool) -> Self {
        if json_flag || !std::io::stdout().is_terminal() {
            OutputFormat::Json
        } else {
            OutputFormat::Table
        }
    }
}
```

The `--json` flag is still there for humans who want JSON in their terminal. But agents never need to remember it. Piped output is JSON by default. This one decision eliminates an entire class of "the agent forgot the `--json` flag" bugs.

---

## Exit Codes

Every CLI uses the same semantic exit code mapping. An agent reads the code and knows immediately what category of failure occurred.

| Code | Meaning | Agent Action |
|------|---------|-------------|
| `0` | Success | Proceed |
| `1` | Transient error (network, IO, API) | Retry with backoff |
| `2` | Configuration error (missing config, invalid setup) | Fix config, then retry |
| `3` | Input/auth error (bad credentials, invalid input) | Request new credentials or fix input |
| `4` | Rate limited | Wait, then retry |

### Implementation

```rust
// src/errors.rs

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Http(_) | Self::Api { .. } | Self::Io(_) => 1,
            Self::Config(_) | Self::NoProviders(_) => 2,
            Self::AuthMissing { .. } | Self::InvalidInput(_) => 3,
            Self::RateLimited { .. } => 4,
        }
    }

    pub fn error_code(&self) -> &str {
        match self {
            Self::Http(_) => "http_error",
            Self::Api { .. } => "api_error",
            Self::Config(_) => "config_error",
            Self::AuthMissing { .. } => "auth_missing",
            Self::RateLimited { .. } => "rate_limited",
            Self::InvalidInput(_) => "invalid_input",
            Self::Io(_) => "io_error",
            Self::NoProviders(_) => "no_providers",
        }
    }

    pub fn suggestion(&self) -> String {
        match self {
            Self::AuthMissing { provider } => format!(
                "Set {}_KEY env var or add to config file",
                provider.to_uppercase()
            ),
            Self::RateLimited { retry_after, .. } => format!(
                "Rate limited. Retry after {} seconds",
                retry_after.unwrap_or(60)
            ),
            Self::Config(msg) => format!("Check config file: {msg}"),
            _ => "See --help for usage".into(),
        }
    }
}
```

### Main entry point

```rust
fn main() {
    let cli = Cli::parse();
    let format = OutputFormat::detect(cli.json);

    match run(cli, format) {
        Ok(()) => {}
        Err(e) => {
            output::render_error(format, e.error_code(), &e.to_string(), &e.suggestion());
            std::process::exit(e.exit_code());
        }
    }
}
```

---

## Agent Info

The `agent-info` subcommand is a machine-readable capability manifest. An agent calls it once, learns everything the tool can do, and operates it without reading documentation.

```bash
$ search agent-info
```

```json
{
  "name": "search",
  "version": "0.4.2",
  "description": "Multi-provider search CLI",
  "commands": ["search", "config show", "config set", "agent-info", "update"],
  "modes": ["auto", "general", "news", "academic", "people", "deep"],
  "providers": [
    { "name": "brave", "configured": true, "capabilities": ["general", "news"] },
    { "name": "exa", "configured": false, "capabilities": ["academic", "similar"] }
  ],
  "env_prefix": "SEARCH_",
  "config_path": "~/.config/search/config.toml",
  "output_formats": ["json", "table"],
  "auto_json_when_piped": true
}
```

The `providers` array includes runtime state -- which providers are actually configured on this machine, not just which ones exist. An agent can check capabilities before making a call, instead of trying and failing.

Some CLIs go further. xmaster embeds the X/Twitter algorithm weights, optimal posting hours, and engagement multipliers directly in agent-info. The agent doesn't just learn how to use the tool. It learns how to use the tool well.

```json
{
  "name": "xmaster",
  "version": "1.0.0",
  "algorithm": {
    "weights": [
      { "signal": "like", "weight": 0.5 },
      { "signal": "reply", "weight": 1.0 },
      { "signal": "retweet", "weight": 13.5 },
      { "signal": "thread_read_time", "weight": 150.0 }
    ],
    "media_hierarchy": ["native_video", "images", "gifs", "links"],
    "best_posting_hours": "9-11 AM local time"
  },
  "usage_hints": [
    "Always run 'xmaster analyze' before posting",
    "Never put external links in main tweet -- put in first reply",
    "Reply to commenters -- conversations worth 150x a like"
  ]
}
```

---

## Skill Installation

A skill is a Markdown file that teaches an agent how to use the CLI -- not just the commands, but the workflows, strategies, and pitfalls. The CLI itself installs these files.

### The problem with external skills

If the skill lives in a separate repo, it drifts. The CLI ships v2, the skill still describes v1 behaviour. Nobody remembers to update both. So we embed the skill in the binary and ship them together.

### Two strategies

**Strategy A: Per-platform templates** (autoresearch)

Each agent platform has different YAML frontmatter requirements. Claude Code supports `argument-hint` and `user-invocable`. Gemini accepts only `name` and `description`. So the CLI generates platform-specific variants:

```rust
pub fn skill_md(platform: &str) -> String {
    let body = core_skill_body();  // Shared content
    let desc = "Autonomous experiment loop...";

    match platform {
        "claude-code" => format!(r#"---
name: autoresearch
description: >
  {desc}
argument-hint: "[goal or metric]"
user-invocable: true
metadata:
  version: "{version}"
---
{body}"#),

        "gemini" | "codex" | _ => format!(r#"---
name: autoresearch
description: >
  {desc}
---
{body}"#),
    }
}
```

Install to all platforms at once:

```bash
$ autoresearch install claude-code
$ autoresearch install gemini
$ autoresearch install codex
$ autoresearch install all  # every platform
```

**Strategy B: Write-once, symlink everywhere** (xmaster)

The skill content is identical across platforms. Write it to a canonical location (`~/.agents/skills/`), then symlink from each platform's skill directory:

```rust
const SKILL_CONTENT: &str = include_str!("../../skill/SKILL.md");

fn install_skill() -> Result<InstallResult, CliError> {
    let targets = skill_targets();
    let primary = targets.iter().find(|t| t.is_primary).unwrap();

    // Write to canonical location
    std::fs::create_dir_all(&primary.path)?;
    std::fs::write(primary.path.join("SKILL.md"), SKILL_CONTENT)?;

    // Symlink from platform-specific directories
    for target in targets.iter().filter(|t| !t.is_primary) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(&primary_skill_path, &target_skill)?;
        #[cfg(not(unix))]
        std::fs::write(&target_skill, SKILL_CONTENT)?;  // Windows fallback
    }
}
```

The symlink approach means one update touches every platform. No drift between agents.

### Skill directories

| Platform | Path |
|----------|------|
| Universal | `~/.agents/skills/<tool>/SKILL.md` |
| Claude Code | `~/.claude/skills/<tool>/SKILL.md` |
| Codex CLI | `~/.codex/skills/<tool>/SKILL.md` |
| Gemini CLI | `~/.gemini/skills/<tool>/SKILL.md` |
| Cursor | `.cursor/skills/<tool>/SKILL.md` |
| Windsurf | `.windsurf/skills/<tool>/SKILL.md` |

### Version detection

Before writing, check if the installed skill is already current:

```rust
if file_path.exists() {
    if let Ok(existing) = fs::read_to_string(&file_path) {
        if existing.contains(&format!("version: {}", env!("CARGO_PKG_VERSION"))) {
            return Err(CliError::AlreadyInstalled(path));
        }
    }
}
```

### Skill status

```bash
$ xmaster skill status

Platform                    Path                                    Installed  Current
Universal (.agents)         ~/.agents/skills/xmaster/SKILL.md       Yes        Yes
Claude Code                 ~/.claude/skills/xmaster/SKILL.md       Yes        Yes
Codex CLI                   ~/.codex/skills/xmaster/SKILL.md        Yes        Outdated
Gemini CLI                  ~/.gemini/skills/xmaster/SKILL.md       No         -
                                                                    Run: xmaster skill update
```

---

## Self-Update

The binary updates itself from GitHub Releases. When it updates, the bundled skill updates too -- because the skill is compiled into the binary.

```rust
// src/commands/update.rs

pub async fn execute(check: bool) -> Result<(), CliError> {
    let current = env!("CARGO_PKG_VERSION");

    let status = self_update::backends::github::Update::configure()
        .repo_owner("199-biotechnologies")
        .repo_name("your-cli")
        .bin_name("your-cli")
        .current_version(current)
        .build()?;

    if check {
        let latest = status.get_latest_release()?;
        let latest_ver = latest.version.trim_start_matches('v');
        if latest_ver == current {
            println!("Already up to date (v{current})");
        } else {
            println!("Update available: v{current} -> v{latest_ver}");
            println!("Run `your-cli update` to install");
        }
    } else {
        let result = status.update()?;
        println!("Updated: v{current} -> v{}", result.version());
    }
    Ok(())
}
```

The update chain:

1. `your-cli update` -- pulls latest binary from GitHub Releases
2. New binary contains new `SKILL_CONTENT` (compiled in via `include_str!`)
3. `your-cli skill update` -- re-deploys bundled skill to all agent platforms
4. Every agent picks up the new instructions on next invocation

Binary and skill are always in sync. No version skew.

---

## The Standard Crate Stack

Every CLI starts from the same dependencies. No surprises, no exotic choices.

| Layer | Crate | Why |
|-------|-------|-----|
| Args | `clap` 4.5 (derive) | Structured parsing with env var overrides built in |
| Async | `tokio` | `JoinSet` for parallel API calls |
| Allocator | `mimalloc` | Shaves 20-30% off peak memory |
| JSON | `serde` + `serde_json` | Envelope format, config, agent-info |
| Database | `rusqlite` (bundled) | SQLite with WAL mode. Zero external deps |
| Tables | `comfy-table` | Human-readable output when TTY detected |
| Colour | `owo-colors` | Terminal colours with auto-detection |
| Errors | `thiserror` | Derive-based error types with exit codes |
| HTTP | `reqwest` 0.12 | When the CLI talks to APIs |
| Config | TOML + env vars | `figment` for complex layering, manual for simple CLIs |
| Update | `self_update` | GitHub Releases-based binary updates |
| Logging | `tracing` | Structured JSONL logs to `~/Library/Application Support/` |

Binary size lands between 1MB and 6MB. Startup under 2ms. No runtime dependencies. No Python, no Node, no Docker.

---

## Putting It Together

A minimal agent-friendly CLI needs five files:

```
src/
  main.rs          # Parse args, detect format, dispatch, handle errors
  errors.rs        # Error types with exit_code(), error_code(), suggestion()
  output/
    mod.rs         # OutputFormat::detect(json_flag)
    json.rs        # render(), render_error() with envelope
    table.rs       # render() with comfy-table
  commands/
    agent_info.rs  # Machine-readable capability manifest
    skill.rs       # install / update / status
    update.rs      # self_update from GitHub Releases
```

The `main.rs` pattern is the same across every CLI:

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "your-cli", version)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

fn main() {
    let cli = Cli::parse();
    let format = OutputFormat::detect(cli.json);

    let code = match run(cli, format) {
        Ok(()) => 0,
        Err(e) => {
            output::render_error(format, &e);
            e.exit_code()
        }
    };
    std::process::exit(code);
}
```

---

## Production Examples

These CLIs run in production using this architecture:

| CLI | Purpose | Repo |
|-----|---------|------|
| [search-cli](https://github.com/199-biotechnologies/search-cli) | 11 providers, 14 search modes, one binary | `cargo install agent-search` |
| [autoresearch](https://github.com/199-biotechnologies/autoresearch-cli) | Autonomous experiment loops for any metric | `cargo install autoresearch` |
| [xmaster](https://github.com/199-biotechnologies/xmaster) | X/Twitter CLI with dual backends | `cargo install xmaster` |
| [labparse](https://github.com/199-biotechnologies/labparse-cli) | Parse lab results into structured biomarker JSON | `brew install labparse` |
| [labassess](https://github.com/199-biotechnologies/labassess-cli) | Score biomarkers against longevity-optimal ranges | `brew install labassess` |

All distributed via `brew tap 199-biotechnologies/tap`.

---

## License

MIT -- Copyright (c) 2025-2026 Boris Djordjevic, [199 Biotechnologies](https://199.bio)

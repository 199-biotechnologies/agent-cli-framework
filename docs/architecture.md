# Architecture Reference

Detailed implementation patterns for agent-friendly Rust CLIs. The [README](../README.md) covers what and why. This document covers how -- the actual code patterns, decision rationale, and edge cases we hit in production.

---

## Table of Contents

- [Output Format Detection](#output-format-detection)
- [The JSON Envelope](#the-json-envelope)
- [Error Architecture](#error-architecture)
- [Agent Info Design](#agent-info-design)
- [Skill System](#skill-system)
- [Self-Update Pipeline](#self-update-pipeline)
- [Crate Selection Rationale](#crate-selection-rationale)
- [Project Structure](#project-structure)

---

## Output Format Detection

The detection logic is three lines. It handles every case:

```rust
pub fn detect(json_flag: bool, csv_flag: bool) -> OutputFormat {
    if csv_flag {
        OutputFormat::Csv
    } else if json_flag || !std::io::stdout().is_terminal() {
        OutputFormat::Json
    } else {
        OutputFormat::Table
    }
}
```

`IsTerminal` is stable in Rust's standard library since 1.70. No crate dependency needed.

### Why not detect on stderr too?

Agents typically capture stdout for data and let stderr pass through to logs. Detecting only on stdout means error messages can still print human-readable text to stderr while data goes out as JSON. If the agent captures both streams, the JSON envelope on stdout is self-contained and parseable regardless of what stderr contains.

### The CSV escape hatch

Some CLIs (xmaster) support `--csv` for spreadsheet workflows. This is rare. Most CLIs need only JSON and Table. Add CSV when users ask for it, not before.

---

## The JSON Envelope

### Envelope v1 contract

Every response has exactly these top-level fields:

```typescript
// Pseudo-schema
{
  version: "1",                          // String, not number. Enables "1.1" later
  status: "success" | "error",
  data?: object,                         // Present on success
  error?: { code, message, suggestion }, // Present on error
  metadata?: object                      // Optional runtime stats
}
```

Agents parse `status` first, then branch. They never need to guess whether a response is an error by checking for missing fields.

### Metadata conventions

Metadata is optional and freeform, but we use consistent keys:

| Key | Type | Purpose |
|-----|------|---------|
| `elapsed_ms` | `u128` | Wall-clock time for the operation |
| `markers_found` | `usize` | Count of primary results |
| `parser` | `String` | Which internal parser handled the input |
| `provider` | `String` | Which external API served the request |
| `cached` | `bool` | Whether the result came from cache |

Agents can use `elapsed_ms` to detect performance regressions. `cached` tells them whether the data is fresh.

### Error code vocabulary

Stick to snake_case strings. These are the codes that appear across our production CLIs:

```
config_error        -- Config file missing, malformed, or invalid
auth_missing        -- API key not set for a required provider
rate_limited        -- Upstream API returned 429
http_error          -- Network failure, DNS resolution, timeout
api_error           -- Upstream returned non-2xx (not rate limit)
json_error          -- Response couldn't be parsed as JSON
io_error            -- File read/write failure
invalid_input       -- User/agent provided bad input
not_found           -- Requested resource doesn't exist
no_providers        -- No search/API providers configured
parse_error         -- Input data couldn't be parsed
already_installed   -- Skill already at current version (exit 0)
```

### The suggestion field

This is the single most important field for agent consumption. Rules:

1. Be specific. "Set SEARCH_BRAVE_KEY env var or add `brave_key` to ~/.config/search/config.toml" beats "check your configuration."
2. Include the exact command or env var name. Agents follow suggestions literally.
3. For rate limits, include the retry delay: "Retry after 60 seconds."
4. For auth errors, name the provider: "Set XMASTER_KEYS__API_KEY for Twitter API access."

---

## Error Architecture

### Error enum with thiserror

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error from {provider}: {message}")]
    Api { provider: String, message: String, status: u16 },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("No API key for {provider}")]
    AuthMissing { provider: String },

    #[error("Rate limited by {provider}")]
    RateLimited { provider: String, retry_after: Option<u64> },

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### The three methods

Every error variant implements three methods. These are not optional:

```rust
impl CliError {
    /// Semantic exit code for the process
    pub fn exit_code(&self) -> i32 { ... }

    /// Machine-readable error identifier (snake_case)
    pub fn error_code(&self) -> &str { ... }

    /// Actionable fix instruction for agents
    pub fn suggestion(&self) -> String { ... }
}
```

If you add a new error variant and forget the suggestion, the agent gets "See --help for usage" -- which is useless. Write the suggestion first. If you can't explain how to fix an error, reconsider whether it should be a separate variant.

### Exit code assignment

The mapping is mechanical:

- Network flakiness, IO, API timeouts -- **1** (transient, retry)
- Bad config files, missing features -- **2** (needs human/agent config fix)
- Bad credentials, bad input data -- **3** (needs different input)
- Rate limiting specifically -- **4** (needs wait, then retry)

Code 4 is separated from 1 because the retry strategy differs. A transient error wants exponential backoff. A rate limit wants a fixed delay (often provided in the `retry_after` header).

---

## Agent Info Design

### Levels of richness

Start minimal. Add depth as the CLI matures.

**Level 1 -- Inventory** (every CLI should have this):

```json
{
  "name": "your-cli",
  "version": "0.1.0",
  "commands": ["search", "config", "agent-info"],
  "output_formats": ["json", "table"],
  "auto_json_when_piped": true
}
```

**Level 2 -- Runtime state** (when the CLI has configurable providers/backends):

```json
{
  "providers": [
    { "name": "brave", "configured": true, "capabilities": ["general", "news"] },
    { "name": "exa", "configured": false, "capabilities": ["academic"] }
  ],
  "env_prefix": "SEARCH_",
  "config_path": "~/.config/search/config.toml"
}
```

**Level 3 -- Domain knowledge** (when the tool operates in a domain with non-obvious best practices):

```json
{
  "algorithm": { ... },
  "usage_hints": [
    "Always run 'xmaster analyze' before posting",
    "Conversations worth 150x a like"
  ],
  "best_practices": {
    "experiment_order": ["Hyperparameters first", "Regularization second"],
    "when_stuck": ["After 5+ consecutive discards, change strategy"]
  }
}
```

Level 3 is where agent-info stops being a manifest and starts being a compressed skill. xmaster's agent-info includes Twitter algorithm weights. autoresearch's includes Karpathy's experiment-ordering strategy. The agent gets actionable domain knowledge from a single JSON call.

### Implementation pattern

```rust
// src/commands/agent_info.rs

pub fn execute() {
    let info = serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "commands": commands_list(),
        "capabilities": runtime_capabilities(),
        "env_prefix": "YOUR_CLI_",
        "config_path": config::config_path().display().to_string(),
        "output_formats": ["json", "table"],
        "auto_json_when_piped": true,
    });

    // agent-info always outputs JSON, regardless of format detection
    println!("{}", serde_json::to_string_pretty(&info).unwrap());
}
```

Note: agent-info always prints JSON. Never tables. The whole point is machine readability.

---

## Skill System

### Architecture decision: embed vs external

We tried both. External skill repos drift within weeks. Embedded skills stay in sync with the binary because they're compiled into it.

**Embedding with `include_str!`:**

```rust
const SKILL_CONTENT: &str = include_str!("../../skill/SKILL.md");
```

The skill file lives in the repo at `skill/SKILL.md`. It's compiled into the binary. When the binary ships, the skill ships. One version number. No coordination.

**Embedding with format strings:**

```rust
fn core_skill_body() -> String {
    format!(r##"
## Your CLI -- Skill Content

Version: {version}
...
"##, version = env!("CARGO_PKG_VERSION"))
}
```

This approach lets you inject the version dynamically. Use it when the skill content references the current version or has platform-conditional sections.

### Platform frontmatter adaptation

Each agent platform parses skill frontmatter differently:

| Platform | Supported fields |
|----------|-----------------|
| Claude Code | `name`, `description`, `argument-hint`, `user-invocable`, `metadata` |
| Gemini CLI | `name`, `description` only |
| Codex CLI | `name`, `description` only |
| GitHub Copilot | `name`, `description`, `argument-hint`, `user-invocable` |
| Cursor | `name`, `description`, `metadata` |
| Windsurf | `name`, `description` only |

If your skill uses Claude Code-specific fields (like `argument-hint`), you need per-platform templates. If it uses only `name` and `description`, the write-once-symlink approach is simpler.

### The symlink strategy

xmaster writes the canonical file to `~/.agents/skills/xmaster/SKILL.md`, then symlinks from platform-specific directories. One file, multiple entry points.

```
~/.agents/skills/xmaster/SKILL.md     (canonical, written)
~/.claude/skills/xmaster/SKILL.md     -> symlink to above
~/.codex/skills/xmaster/SKILL.md      -> symlink to above
~/.gemini/skills/xmaster/SKILL.md     -> symlink to above
```

Edge cases handled:
- **Existing symlink pointing elsewhere** -- remove and recreate
- **Existing regular file with matching content** -- report "already_current", don't touch
- **Existing regular file with old content** -- replace with symlink
- **Symlink creation fails** (permissions, Windows) -- fall back to file copy
- **Directory doesn't exist** -- create it, or skip with a message

### Skill update lifecycle

```
Binary v1.0 ships with Skill v1.0
  └── `your-cli skill install` writes Skill v1.0 to all platforms

Binary v1.1 ships with Skill v1.1 (updated instructions)
  └── `your-cli update` pulls new binary
  └── `your-cli skill update` re-deploys Skill v1.1
      └── Overwrites canonical file
      └── Symlinks still point to it -- agents get v1.1 immediately
```

### Skill status reporting

```rust
pub async fn status(format: OutputFormat) -> Result<(), CliError> {
    let targets = skill_targets();
    let mut needs_update = false;

    for target in &targets {
        let skill_path = target.path.join("SKILL.md");
        let installed = skill_path.exists();
        let current = if installed {
            std::fs::read_to_string(&skill_path)
                .map(|c| c == SKILL_CONTENT)  // Compare against bundled
                .unwrap_or(false)
        } else {
            false
        };
        if installed && !current {
            needs_update = true;
        }
    }
}
```

The comparison is a full content match against the embedded `SKILL_CONTENT`. Not a version string check. This catches any manual edits or corruption.

---

## Self-Update Pipeline

### Binary update via `self_update` crate

```rust
self_update::backends::github::Update::configure()
    .repo_owner("199-biotechnologies")
    .repo_name("your-cli")
    .bin_name("your-cli")
    .current_version(env!("CARGO_PKG_VERSION"))
    .build()?;
```

The crate downloads the matching release asset for the current platform (target triple), replaces the running binary, and reports the version change.

### Check-only mode

```bash
$ your-cli update --check
Update available: v0.3.1 -> v0.4.0
Run `your-cli update` to install
```

Agents can check without committing. Useful for notification workflows.

### CI release workflow

Tag a release in GitHub. CI builds binaries for:
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`

Upload as release assets. The `self_update` crate matches the current platform and downloads the right one.

### Homebrew tap update

Separately, update the formula in `199-biotechnologies/homebrew-tap` with the new version and sha256. Users on Homebrew get updates via `brew upgrade`.

Two distribution channels. Neither depends on the other.

---

## Crate Selection Rationale

### Why `mimalloc`?

Default Rust allocator is fine for most programs. But CLIs that make many small allocations (JSON parsing, string manipulation) see 20-30% memory reduction with mimalloc. One line to add:

```rust
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

### Why bundled SQLite?

`rusqlite` with the `bundled` feature compiles SQLite from source into the binary. No system SQLite dependency. The binary runs on a fresh machine with zero setup. WAL mode is set at connection time for concurrent read access.

### Why `comfy-table` over `tabled`?

`comfy-table` handles wide Unicode characters correctly and supports dynamic column widths based on terminal size. `tabled` is lighter but misaligns with CJK text.

### Why `owo-colors` over `colored`?

`owo-colors` is zero-allocation. It writes colour codes directly to the formatter. `colored` allocates a `ColoredString` wrapper. For CLIs that print thousands of lines, the difference matters. For a 10-line output, it doesn't. We standardised on `owo-colors` for consistency.

---

## Project Structure

```
your-cli/
├── Cargo.toml
├── skill/
│   └── SKILL.md                    # Skill content (compiled into binary)
├── src/
│   ├── main.rs                     # Entry point: parse, detect format, dispatch, exit
│   ├── cli.rs                      # Clap derive structs
│   ├── errors.rs                   # Error enum with exit_code + error_code + suggestion
│   ├── output/
│   │   ├── mod.rs                  # OutputFormat enum + detect()
│   │   ├── json.rs                 # render() + render_error() with envelope
│   │   └── table.rs                # comfy-table rendering
│   └── commands/
│       ├── mod.rs                  # Command dispatch
│       ├── agent_info.rs           # Machine-readable capabilities
│       ├── skill.rs                # install / update / status
│       ├── update.rs               # self_update from GitHub
│       └── ...                     # Your domain commands
└── tests/
    └── ...
```

The `skill/` directory sits at the repo root, not inside `src/`. It's Markdown, not Rust. Keep it where documentation people can find and edit it.

---

## Checklist for a New CLI

Before shipping, verify:

- [ ] `--json` flag works on every subcommand
- [ ] Piped output is JSON without any flag
- [ ] Every error has an `error_code`, `exit_code`, and `suggestion`
- [ ] `agent-info` returns valid JSON with at minimum name, version, commands
- [ ] `skill install` writes to all supported agent platforms
- [ ] `skill status` reports installed/current/outdated correctly
- [ ] `update --check` reports available version without installing
- [ ] `update` replaces the binary from GitHub Releases
- [ ] Binary runs on a clean machine with no prior setup
- [ ] Binary size is under 10MB
- [ ] Startup is under 10ms

---

*Built by [199 Biotechnologies](https://199.bio). Extracted from production CLIs that agents call thousands of times a day.*

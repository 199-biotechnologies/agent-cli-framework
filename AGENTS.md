# Agent CLI Framework -- Build Instructions for AI Agents

You are building a Rust CLI that AI agents can discover, call, and learn from. Follow these rules exactly. Do not deviate, add features, or invent patterns not described here.

## Spirit

This framework builds tools that are self-explanatory, hyper-efficient, powerful, fast, and local. The binary IS the interface -- no MCP servers, no protocol layers, no external documentation. An agent that has your CLI on PATH has everything it needs.

## Architecture

Split your CLI into focused modules. Never write a monolithic main.rs.

```
src/
  main.rs         # Entry point only: parse, detect format, dispatch, exit
  cli.rs          # Clap derive: Cli struct + Commands enum + Args structs
  config.rs       # AppConfig + load() via figment (3-tier precedence)
  error.rs        # Error enum with exit_code(), error_code(), suggestion()
  output.rs       # Format enum, Ctx struct, print_success_or(), print_error()
  commands/
    mod.rs        # Re-exports
    <command>.rs  # One file per domain command
    agent_info.rs # Capability manifest with arg schemas (always present)
    skill.rs      # Skill install + status (always present)
    config.rs     # config show/path (always present)
    update.rs     # Self-update (optional)
  tests/          # Integration tests verifying contracts
  Cargo.toml
```

## Non-Negotiable Rules

1. **Every stdout path respects output format.** JSON when piped, colored human-readable output in terminal. No exceptions. No raw text leaks.
2. **`--help` and `--version` exit 0.** They are not errors. Wrap in success envelope when piped.
3. **Errors go to stderr.** Both JSON and human-readable. `tool cmd | jq` must never break on error text.
4. **Exit codes are: 0=success, 1=retry, 2=config, 3=input, 4=rate-limited.** Nothing else.
5. **`agent-info` matches reality.** Every command listed works. Every flag described exists. This is a tested contract.
6. **Suggestions are tested instructions.** An agent follows them literally. Wrong suggestions are P0 bugs.
7. **No interactive prompts.** No stdin reads. No pagers. Destructive ops take `--confirm` flag.
8. **Secrets are never displayed in plain text.** Mask with `mask_secret()`. Never include in error messages.

## Output Format

Detect automatically. Bundle format + quiet into an output context:

```rust
pub enum Format { Json, Human }

impl Format {
    pub fn detect(json_flag: bool) -> Self {
        if json_flag || !std::io::stdout().is_terminal() { Format::Json }
        else { Format::Human }
    }
}

pub struct Ctx { pub format: Format, pub quiet: bool }

impl Ctx {
    pub fn new(json_flag: bool, quiet: bool) -> Self {
        Self { format: Format::detect(json_flag), quiet }
    }
}
```

Pass `Ctx` to all commands. `--quiet` suppresses human output; JSON always emits.

Success envelope (stdout):
```json
{"version": "1", "status": "success", "data": { ... }}
```

Error envelope (stderr):
```json
{"version": "1", "status": "error", "error": {"code": "...", "message": "...", "suggestion": "..."}}
```

Extended status values for multi-source operations: `success`, `partial_success`, `all_failed`, `no_results`.

## Error Pattern

Every error enum implements three methods -- the contract that connects errors to exit codes and JSON envelopes:

```rust
impl AppError {
    pub fn exit_code(&self) -> i32;    // 1=transient, 2=config, 3=input, 4=rate-limited
    pub fn error_code(&self) -> &str;  // "invalid_input", "config_error", etc.
    pub fn suggestion(&self) -> &str;  // Tested recovery instruction (agents follow literally)
}
```

Standard categories: `InvalidInput` (3), `Config` (2), `Transient`/`Io`/`Update` (1), `RateLimited` (4).

## Entry Point Pattern

Pre-scan `--json` before clap parses so it works on help/version/error paths. Never let clap own the exit code — always exit explicitly through the framework.

```rust
fn has_json_flag() -> bool {
    std::env::args_os().any(|a| a == "--json")
}

fn main() {
    let json_flag = has_json_flag();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            if matches!(e.kind(),
                clap::error::ErrorKind::DisplayHelp
                | clap::error::ErrorKind::DisplayVersion
            ) {
                let format = Format::detect(json_flag);
                match format {
                    Format::Json => { print_help_json(e); std::process::exit(0); }
                    Format::Human => e.exit(),
                }
            }
            // Parse errors: we own the exit code, always 3
            let format = Format::detect(json_flag);
            print_clap_error(format, &e);
            std::process::exit(3);
        }
    };
    let ctx = Ctx::new(cli.json, cli.quiet);
    let config = config::load().unwrap_or_else(|e| {
        print_error(ctx.format, &e);
        std::process::exit(e.exit_code());
    });
    if let Err(e) = run(cli, ctx, &config) {
        print_error(ctx.format, &e);
        std::process::exit(e.exit_code());
    }
}
```

## Config Convention

- Path: `~/.config/<app>/config.toml`
- Load: defaults -> TOML file -> env vars (prefix `<APP>_`)
- Use `figment` crate for merging
- Use `directories` crate for platform paths

## Secret Convention

- Resolution: flag value -> env var -> config file (first non-empty wins)
- Display: always masked (`sk-pr...1234`)
- Never store in state databases, never log plain text

## Directory Convention

| Purpose | Path | Deletable? |
|---------|------|-----------|
| Config | `~/.config/<app>/` | No (user settings) |
| State | `~/.local/share/<app>/` | Careful (operational data) |
| Cache | `~/.cache/<app>/` | Always safe |

## Command Naming

Always alias CRUD subcommands: `list`/`ls`, `create`/`new`, `delete`/`rm`, `show`/`get`. Use `#[command(visible_alias = "ls")]`. Be consistent across all subcommand groups.

## Standard Commands

Every CLI has these built-in commands:
- `agent-info` (alias `info`) -- capability manifest, raw JSON, not wrapped in envelope
- `skill install` -- write SKILL.md to `~/.claude/skills/<name>/`, `~/.codex/skills/<name>/`, `~/.gemini/skills/<name>/`
- `skill status` -- check installation status

Standard:
- `config show` -- display effective merged config (secrets masked)
- `config path` -- print config file path

Optional:
- `update [--check]` -- self-update from GitHub Releases

## Global Flags

Always at the top-level `Cli` struct:
- `--json` -- force JSON output even in terminal (required, always present)
- `--quiet` -- suppress informational human output; JSON always emits (required, always present)

## Dependencies

```toml
clap = { version = "4", features = ["derive", "env"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
comfy-table = "7"
owo-colors = "4"
directories = "6"
figment = { version = "0.10", features = ["toml", "env"] }

[profile.release]
lto = true
codegen-units = 1
strip = true
opt-level = 3
```

## Reference

See the `example/` directory in this repo for a working implementation of the five core patterns and the entry point, error type, and output helpers. Config loading, secret handling, XDG paths, and HTTP retry are documented as code patterns in the README's Reusable Modules section.

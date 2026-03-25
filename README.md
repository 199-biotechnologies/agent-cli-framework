<h1 align="center">agent-cli-framework</h1>

<p align="center">
  <strong>A CLI that describes itself is better than a CLI with documentation.</strong><br>
  <em>Patterns for building Rust CLIs that AI agents use instinctively.</em>
</p>

<p align="center">
  <a href="#the-idea">The Idea</a> &middot;
  <a href="#the-patterns">The Patterns</a> &middot;
  <a href="#the-example">The Example</a> &middot;
  <a href="#production-clis">Production CLIs</a>
</p>

---

## The Idea

The best documentation for an AI agent is no documentation.

A CLI should describe itself well enough that an agent can pick it up and use it without reading a README, a wiki, or a skill file. The binary carries its own capability manifest (`agent-info`), its own error recovery hints (`suggestion` fields), and its own output contract (JSON envelope). An agent calls `tool agent-info`, gets back structured JSON with every command, flag, and exit code, and starts working.

The skill file -- the thing that agent platforms like Claude Code, Codex, and Gemini use to discover tools -- should be almost empty. A few lines: "this tool exists, here's what it does, run `agent-info` for the rest." The CLI installs that tiny pointer into every agent platform with a single command. When the binary updates, the pointer updates with it, because it's compiled into the binary.

No separate documentation to maintain. No skill that drifts from the tool it describes. The CLI is the source of truth.

This repo has one working Rust example that you can build and run. It shows every pattern. Extracted from production CLIs at [199 Biotechnologies](https://github.com/199-biotechnologies).

---

## The Patterns

**Four things make a CLI agent-friendly:**

**1. `agent-info` command.** Returns a JSON manifest of everything the tool can do. Commands, flags, exit codes, environment variables. An agent calls this once and operates the tool from memory.

**2. Structured output.** JSON envelope on stdout when piped, coloured table when in a terminal. Auto-detected via `std::io::IsTerminal`. Errors include a `suggestion` field that tells the agent exactly what to do next.

**3. Semantic exit codes.** `0` success, `1` transient (retry), `2` config (fix setup), `3` bad input (fix args), `4` rate limited (wait). The agent reads the code and knows its next move without parsing the error message.

**4. Skill self-install.** The binary carries a minimal SKILL.md compiled in via `include_str!`. One command writes it to `~/.claude/skills/`, `~/.codex/skills/`, `~/.gemini/skills/`, and anywhere else agents look. Binary update = skill update. No drift.

That's it. No framework to install, no traits to implement. Just conventions in a single-file Rust CLI.

---

## The Example

A working Rust CLI that demonstrates all four patterns in ~300 lines:

```
example/
  Cargo.toml
  src/main.rs
```

Build and try it:

```bash
cd example
cargo build --release

# Human mode (coloured output in terminal)
./target/release/greeter hello Boris --style pirate

# Agent mode (pipe triggers JSON automatically)
./target/release/greeter hello Boris | jq

# Capability discovery
./target/release/greeter agent-info

# Error with semantic exit code and suggestion
./target/release/greeter hello ""
echo $?  # exits 3 (bad input)

# Install skill to all agent platforms
./target/release/greeter skill install

# Check what's installed where
./target/release/greeter skill status

# Self-update from GitHub Releases
./target/release/greeter update --check
```

### What `agent-info` returns

```json
{
  "name": "greeter",
  "version": "0.1.0",
  "description": "Minimal agent-friendly CLI example",
  "commands": {
    "hello <name>": "Greet someone. Styles: friendly, formal, pirate.",
    "agent-info": "This manifest.",
    "skill install": "Install skill file to agent platforms.",
    "update": "Self-update binary from GitHub Releases."
  },
  "exit_codes": {
    "0": "Success",
    "1": "Transient error (IO, network) — retry",
    "3": "Bad input — fix arguments"
  },
  "auto_json_when_piped": true
}
```

### What the skill file looks like

The entire skill, installed to every agent platform, is this:

```yaml
---
name: greeter
description: >
  Greet people in different styles. Run `greeter agent-info` for full
  capabilities, flags, and exit codes.
---

## greeter

A demo CLI. Run `greeter agent-info` for the machine-readable capability
manifest. Run `greeter hello <name> --style pirate` to use it.
```

That's it. The skill is a signpost. The binary is the manual.

### What errors look like

```json
{
  "version": "1",
  "status": "error",
  "error": {
    "code": "invalid_input",
    "message": "Invalid input: name cannot be empty",
    "suggestion": "Check the --help output for valid arguments"
  }
}
```

The agent reads `code`, checks the exit code (3 = bad input), follows the `suggestion`. No guessing.

---

## Production CLIs

These CLIs use these patterns in production:

| CLI | What it does | Install |
|-----|-------------|---------|
| [search-cli](https://github.com/199-biotechnologies/search-cli) | 11 search providers, 14 modes, one binary | `cargo install agent-search` |
| [autoresearch](https://github.com/199-biotechnologies/autoresearch-cli) | Autonomous experiment loops for any metric | `cargo install autoresearch` |
| [xmaster](https://github.com/199-biotechnologies/xmaster) | X/Twitter CLI with dual backends | `cargo install xmaster` |
| [labparse](https://github.com/199-biotechnologies/labparse-cli) | Lab results to structured biomarker JSON | `brew install labparse` |
| [labassess](https://github.com/199-biotechnologies/labassess-cli) | Score biomarkers against longevity-optimal ranges | `brew install labassess` |

All available via `brew tap 199-biotechnologies/tap`.

---

## License

MIT -- Copyright (c) 2025-2026 Boris Djordjevic, [199 Biotechnologies](https://199.bio)

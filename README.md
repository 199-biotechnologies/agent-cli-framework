<h1 align="center">agent-cli-framework</h1>

<p align="center">
  <strong>Build CLIs that agents use instinctively. No MCP server. No skill files. Just a binary.</strong><br>
  <em>Patterns extracted from production tools at <a href="https://github.com/199-biotechnologies">199 Biotechnologies</a>.</em>
</p>

<p align="center">
  <a href="#why-clis">Why CLIs</a> &middot;
  <a href="#four-patterns">Four Patterns</a> &middot;
  <a href="#the-example">The Example</a> &middot;
  <a href="#production-clis">Production CLIs</a>
</p>

---

## Why CLIs

Something odd happened over the past year.

MCP servers were supposed to be the way AI agents talk to the world. Structured schemas, typed parameters, proper JSON-RPC. The protocol looked right on paper. Then people started using it at scale.

Scalekit ran 75 benchmarked tasks comparing MCP against plain CLI calls. The simplest task -- fetching a repo's language and licence -- cost **1,365 tokens via CLI** and **44,026 via MCP**. That's a 32x overhead. Across all tasks, MCP used 4-32x more tokens, cost 17x more at scale, and failed 28% of the time due to server timeouts. CLI succeeded every time.

The reason is structural. Each MCP tool definition burns 550-1,400 tokens of context just to describe itself. A typical MCP setup dumps 55,000 tokens into the context window before the agent does anything useful. One team reported three MCP servers consuming 143,000 of 200,000 available tokens -- 72% of the agent's working memory gone on tool descriptions alone.

Then there's the confusion problem. Speakeasy tested tool counts against model accuracy. At 20 tools, large models scored 19 out of 20. At 107 tools, both large and small models failed completely. GitHub Copilot cut from 40 tools to 13 and saw measurable improvements. Block rebuilt its Linear MCP server three times, going from 30+ tools down to 2.

Meanwhile, agents were already using CLIs without being asked. Eugene Petrenko at JetBrains documented how AI agents autonomously discovered and used the `gh` CLI -- handling authentication, reading PR comments, managing issues -- because LLMs have been trained on millions of shell examples. The grammar of `tool subcommand --flag value` is already in the weights.

The pattern that emerged: a single Rust binary with structured JSON output, semantic exit codes, and a built-in capability manifest (`agent-info`) gives an agent everything it needs. No server process to manage. No schema dump eating the context window. No protocol layer between the agent and the work.

A CLI is ~80 tokens of agent prompt plus a 50-200 token `--help` call when needed. An MCP server is 55,000 tokens upfront whether you need them or not.

This repo shows how to build that kind of CLI.

---

## Four Patterns

A CLI becomes agent-friendly with four additions:

**`agent-info` command.** A JSON manifest of everything the tool can do -- commands, flags, exit codes, environment variables. The agent calls it once and works from memory. This replaces documentation. The binary describes itself.

**Structured output.** JSON envelope on stdout when piped, coloured table when in a terminal. Auto-detected via `std::io::IsTerminal`. Errors include a `suggestion` field telling the agent exactly how to recover.

**Semantic exit codes.** `0` success, `1` transient (retry), `2` config (fix setup), `3` bad input (fix args), `4` rate limited (wait). The agent reads the code and knows its next move without parsing the error message.

**Skill self-install.** The binary carries a minimal SKILL.md compiled in via `include_str!`. One command writes it to `~/.claude/skills/`, `~/.codex/skills/`, `~/.gemini/skills/`. The skill is just a signpost -- a few lines saying "this tool exists, run `agent-info` for the rest." Binary update = skill update. No drift.

---

## The Example

A working Rust CLI demonstrating all four patterns in one file:

```
example/
  Cargo.toml
  src/main.rs    (~280 lines)
```

Build and run:

```bash
cd example && cargo build --release

# Human at a terminal -- coloured output
./target/release/greeter hello Boris --style pirate

# Agent piping -- auto-switches to JSON
./target/release/greeter hello Boris | jq

# Capability discovery -- the whole point
./target/release/greeter agent-info

# Error with semantic exit code
./target/release/greeter hello ""
echo $?  # 3 (bad input)

# Install skill to all agent platforms
./target/release/greeter skill install

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

The entire skill, installed to every agent platform:

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

The skill is a signpost. The binary is the manual.

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

Exit code 3. The agent reads `code`, reads `suggestion`, acts.

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

## Further Reading

- [MCP vs CLI: Benchmarking AI Agent Cost & Reliability](https://www.scalekit.com/blog/mcp-vs-cli-use) -- Scalekit
- [Your MCP Server Is Eating Your Context Window](https://www.apideck.com/blog/mcp-server-eating-context-window-cli-alternative) -- Apideck
- [CLI Is the New API and MCP](https://jonnyzzz.com/blog/2026/02/20/cli-tools-for-ai-agents/) -- Eugene Petrenko, JetBrains
- [Reducing MCP Token Usage by 100x](https://www.speakeasy.com/blog/how-we-reduced-token-usage-by-100x-dynamic-toolsets-v2) -- Speakeasy

---

## Licence

MIT -- Copyright (c) 2025-2026 Boris Djordjevic, [199 Biotechnologies](https://199.bio)

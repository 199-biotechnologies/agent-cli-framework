<h1 align="center">agent-cli-framework</h1>

<p align="center">
  <strong>Build CLIs that agents use instinctively. No MCP server. No skill files. Just a binary.</strong><br>
  <em>Patterns extracted from production tools at <a href="https://github.com/199-biotechnologies">Paperfoot AI (SG) Pte. Ltd.</a></em>
</p>

<p align="center">
  <a href="#why-clis">Why CLIs</a> &middot;
  <a href="#five-patterns">Five Patterns</a> &middot;
  <a href="#the-example">The Example</a> &middot;
  <a href="#production-clis">Production CLIs</a>
</p>

---

## Why CLIs

Agents need tools. Not connections to tools. Not descriptions of tools. Actual tools they can pick up and use.

An MCP server is a connection. It tells the agent "there's a service over there, here's its schema, here's how to call it." A skill file is an instruction manual. It tells the agent "here's how to do the thing." But neither of them is the thing itself. The agent reads about capabilities without having them. It's the difference between handing someone a hammer and handing them a pamphlet about hammers.

A CLI is the tool. It sits on the machine, does one job, and explains itself when asked. An agent that has `search` on its PATH can search. An agent that has `labparse` can parse lab results. No intermediary, no server process, no protocol layer. The agent shells out, gets structured JSON back, and moves on.

This matters more than the efficiency argument, but the efficiency argument is brutal too. Scalekit benchmarked 75 tasks: the simplest cost **1,365 tokens via CLI** and **44,026 via MCP** -- a 32x overhead. Each MCP tool definition burns 550-1,400 tokens just to describe itself. A typical setup dumps 55,000 tokens into the context window before any real work starts. Speakeasy found that at 107 tools, model accuracy collapsed to zero. GitHub Copilot cut from 40 tools to 13 and got better results.

But the deeper issue isn't tokens. It's that agents already know how to use CLIs. LLMs trained on millions of shell examples from Stack Overflow, GitHub, and man pages have the grammar of `tool subcommand --flag value` baked into their weights. Eugene Petrenko at JetBrains documented agents autonomously discovering and using the `gh` CLI -- handling auth, reading PRs, managing issues -- without being told it existed.

Skills and MCP servers have their place. But the foundation is the tool itself. A well-built CLI with structured output and a built-in capability manifest gives an agent something no amount of documentation can: the ability to just do the thing.

This repo shows how to build that kind of CLI.

---

## Five Patterns

A CLI becomes agent-friendly with five additions:

**`agent-info` command.** A JSON manifest of everything the tool can do -- commands, flags, exit codes, environment variables. The agent calls it once and works from memory. This replaces documentation. The binary describes itself.

**Structured output.** JSON envelope on stdout when piped, coloured table when in a terminal. Auto-detected via `std::io::IsTerminal`. Errors include a `suggestion` field telling the agent exactly how to recover.

**Semantic exit codes.** `0` success, `1` transient (retry), `2` config (fix setup), `3` bad input (fix args), `4` rate limited (wait). The agent reads the code and knows its next move without parsing the error message.

**Skill self-install.** The binary carries a minimal SKILL.md compiled in via `include_str!`. One command writes it to `~/.claude/skills/`, `~/.codex/skills/`, `~/.gemini/skills/`. The skill is just a signpost -- a few lines saying "this tool exists, run `agent-info` for the rest." Binary update = skill update. No drift.

**Distribution and self-update.** Three install paths, one update mechanism. The user gets the binary however they prefer. The binary updates itself.

```
Install paths (pick any):

  brew tap 199-biotechnologies/tap && brew install your-cli    # Homebrew
  cargo install your-cli                                        # crates.io
  curl -fsSL https://your-cli.dev/install.sh | sh              # shell script

Self-update (built into the binary):

  your-cli update --check       # check for new version
  your-cli update               # pull latest from GitHub Releases
  your-cli skill install        # re-deploy updated skill to all agents
```

The Homebrew tap is a GitHub repo (`your-org/homebrew-tap`) with a formula per CLI. When you cut a release, CI builds binaries for `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, and `aarch64-unknown-linux-gnu`, uploads them as release assets, and updates the tap formula with the new version and sha256.

crates.io is `cargo publish`. The binary lands on every machine with a Rust toolchain.

The shell installer is a `curl | sh` one-liner that detects the platform, downloads the right binary from GitHub Releases, and drops it into `/usr/local/bin`.

Self-update uses the [`self_update`](https://crates.io/crates/self_update) crate. It checks GitHub Releases for a newer version, downloads the matching binary, and replaces itself. After update, `your-cli skill install` re-deploys the bundled skill -- which now contains the latest version's instructions. One command updates the tool. One command updates every agent's knowledge of it.

---

## The Example

A working Rust CLI demonstrating all five patterns in one file:

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

## Mistakes We Made

These came from applying the patterns above to production CLIs and watching agents actually use them. Every one of these shipped before we caught it.

**Wrong suggestions.** Our search CLI told agents to set `SEARCH_BRAVE_KEY` when the actual env var was `SEARCH_KEYS_BRAVE`. The agent followed the suggestion exactly, set the wrong variable, and reported auth still broken. Suggestions are not hints. They are instructions. Test them.

**JSON only on the main command.** The primary `search` command returned proper JSON envelopes. But `config show`, `update --check`, and cache-miss paths printed raw text. An agent piping stdout into a JSON parser got a crash instead of data. Every subcommand, every code path, every error -- if it writes to stdout, it must respect the output format.

**Success that was failure.** All eleven providers errored out (rate limits, timeouts, auth). The response: `{"status": "success", "results": []}`. The agent saw success, concluded no results matched the query, and moved on. The actual problem was that nothing worked. We added `partial_success` as a third status for when some providers fail but others return results, and `all_failed` for total failure.

**Retry that never fired.** The retry wrapper caught `Http` transport errors. But providers converted HTTP 500/503 into `Api { message }` errors during response parsing. The retry logic never saw a retryable error. Match on HTTP status codes before you parse the body, not after.

**Dead features in agent-info.** The capability manifest advertised `deep` and `academic` search modes. The functions existed in the code but were never wired into the dispatch path. An agent that called `search --mode deep` got an "unknown mode" error despite agent-info promising it worked. If agent-info says the tool can do something, it must actually do it.

---

## Production CLIs

These CLIs use these patterns in production:

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
- [CLI Is the New API and MCP](https://jonnyzzz.com/blog/2026/02/20/cli-tools-for-ai-agents/) -- Eugene Petrenko, JetBrains
- [Reducing MCP Token Usage by 100x](https://www.speakeasy.com/blog/how-we-reduced-token-usage-by-100x-dynamic-toolsets-v2) -- Speakeasy

---

## Licence

MIT -- Copyright (c) 2025-2026 Boris Djordjevic, Paperfoot AI (SG) Pte. Ltd.

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

Five patterns that turn any Rust CLI into a tool AI agents can pick up and use without documentation, MCP servers, or skill files. The binary describes itself, returns structured output, and uses semantic exit codes. Your CLI becomes the tool, the documentation, and the API -- all in one binary.

[Why This Exists](#why-this-exists) | [Before vs After](#before-vs-after) | [Install](#install) | [How It Works](#how-it-works) | [Features](#features) | [Contributing](#contributing)

</div>

## Why This Exists

Agents need tools. Not connections to tools. Not descriptions of tools. Actual tools they can pick up and use.

An MCP server is a connection -- it tells the agent "there's a service over there, here's its schema, here's how to call it." A skill file is an instruction manual. Neither is the tool itself. The agent reads about capabilities without having them. It's the difference between handing someone a hammer and handing them a pamphlet about hammers.

A CLI is the tool. It sits on the machine, does one job, and explains itself when asked. An agent that has `search` on its PATH can search. An agent that has `labparse` can parse lab results. No intermediary, no server process, no protocol layer. The agent shells out, gets structured JSON back, and moves on.

### The numbers back this up

Scalekit benchmarked 75 tasks: the simplest cost **1,365 tokens via CLI** and **44,026 via MCP** -- a 32x overhead. Each MCP tool definition burns 550-1,400 tokens just to describe itself. A typical setup dumps 55,000 tokens into the context window before any real work starts.

Speakeasy found that at 107 tools, models struggled to select the right one and started hallucinating tool names that didn't exist. GitHub Copilot [cut from 40 tools to 13](https://github.blog/ai-and-ml/github-copilot/how-were-making-github-copilot-smarter-with-fewer-tools/) and got better results.

LLMs already know how to use CLIs. They were trained on millions of shell examples from Stack Overflow, GitHub, and man pages. The grammar of `tool subcommand --flag value` is baked into their weights. Eugene Petrenko at JetBrains documented agents autonomously discovering and using the `gh` CLI -- handling auth, reading PRs, managing issues -- without being told it existed.

This repo gives you the architecture to build CLIs that work that way.

## Before vs After

<table>
<tr>
<th width="50%">Regular CLI</th>
<th width="50%">Agent-Friendly CLI</th>
</tr>
<tr>
<td>

```
$ mytool search "rust cli"
Found 3 results:
  1. Clap framework
  2. Structopt (deprecated)
  3. Argh by Google

$ echo $?
0
```

Human-readable output. Agent has to parse free text. No way to discover capabilities programmatically. Exit code 0 means... it ran?

</td>
<td>

```json
$ mytool search "rust cli" | jq
{
  "version": "1",
  "status": "success",
  "data": {
    "results": [
      {"title": "Clap framework", "url": "..."},
      {"title": "Structopt", "url": "..."},
      {"title": "Argh", "url": "..."}
    ],
    "count": 3
  }
}
```

Structured JSON when piped. Coloured table in a terminal. `agent-info` tells the agent everything it can do. Exit code 3 means "fix your arguments."

</td>
</tr>
</table>

## Install

Clone the repo and build the example:

```bash
git clone https://github.com/199-biotechnologies/agent-cli-framework.git
cd agent-cli-framework/example
cargo build --release
```

Run it:

```bash
# Human at a terminal -- coloured output
./target/release/greeter hello Boris --style pirate

# Agent piping -- auto-switches to JSON
./target/release/greeter hello Boris | jq

# Capability discovery
./target/release/greeter agent-info

# Error with semantic exit code
./target/release/greeter hello ""
echo $?  # 3 (bad input)

# Install skill to all agent platforms
./target/release/greeter skill install
```

## How It Works

```
                    ┌─────────────────────────────────────┐
                    │           Your Rust CLI              │
                    │                                      │
                    │  ┌──────────┐  ┌──────────────────┐  │
  Agent calls       │  │  clap    │  │  Output Format   │  │
  `tool agent-info` │  │  Parser  │  │  Detection       │  │
        │           │  └────┬─────┘  │  (TTY → table)   │  │
        ▼           │       │        │  (Pipe → JSON)   │  │
  ┌───────────┐     │       ▼        └──────────────────┘  │
  │ Capability│     │  ┌─────────┐   ┌──────────────────┐  │
  │ Manifest  │◄────┤  │ Command │   │  JSON Envelope   │  │
  │ (JSON)    │     │  │ Router  │──▶│  { version,      │  │
  └───────────┘     │  └─────────┘   │    status, data } │  │
                    │       │        └──────────────────┘  │
  Agent reads       │       ▼                              │
  exit code ────────┤  ┌─────────┐   ┌──────────────────┐  │
  0: success        │  │ Semantic│   │  Skill            │  │
  1: retry          │  │ Exit    │   │  Self-Install     │  │
  3: fix args       │  │ Codes   │   │  (~/.claude/,     │  │
                    │  └─────────┘   │   ~/.codex/,      │  │
                    │                │   ~/.gemini/)      │  │
                    │                └──────────────────┘  │
                    └─────────────────────────────────────┘
```

## Features

### 1. `agent-info` -- Capability Discovery

The binary describes itself. One command returns a JSON manifest of everything the tool can do: commands, flags, exit codes, environment variables.

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

The agent calls it once and works from memory. This replaces documentation.

### 2. Structured Output -- JSON Envelope

JSON on stdout when piped, coloured table when in a terminal. Auto-detected via `std::io::IsTerminal`. Errors include a `suggestion` field telling the agent exactly how to recover.

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

### 3. Semantic Exit Codes

| Code | Meaning | Agent Action |
|------|---------|-------------|
| `0` | Success | Continue |
| `1` | Transient error (IO, network) | Retry |
| `2` | Config error | Fix setup |
| `3` | Bad input | Fix arguments |
| `4` | Rate limited | Wait and retry |

The agent reads the code and knows its next move without parsing the error message.

### 4. Skill Self-Install

The binary carries a minimal SKILL.md compiled in via `include_str!`. One command writes it to `~/.claude/skills/`, `~/.codex/skills/`, `~/.gemini/skills/`. The skill is just a signpost -- a few lines saying "this tool exists, run `agent-info` for the rest." Binary update = skill update. No drift.

### 5. Distribution and Self-Update

Three install paths, one update mechanism:

```
Install (pick any):
  brew tap your-org/tap && brew install your-cli   # Homebrew
  cargo install your-cli                            # crates.io
  curl -fsSL https://your-cli.dev/install.sh | sh  # shell script

Self-update (built into the binary):
  your-cli update --check      # check for new version
  your-cli update              # pull latest from GitHub Releases
  your-cli skill install       # re-deploy updated skill
```

## Mistakes We Made

These came from shipping CLIs with these patterns and watching agents actually use them. Every one of these went to production before we caught it.

**Wrong suggestions.** Our search CLI told agents to set `SEARCH_BRAVE_KEY` when the actual env var was `SEARCH_KEYS_BRAVE`. The agent followed the suggestion exactly, set the wrong variable, and reported auth still broken. Suggestions are not hints. They are instructions. Test them.

**JSON only on the main command.** The primary `search` command returned proper JSON envelopes. But `config show`, `update --check`, and cache-miss paths printed raw text. An agent piping stdout into a JSON parser got a crash instead of data. Every subcommand, every code path, every error -- if it writes to stdout, it must respect the output format.

**Success that was failure.** All eleven providers errored out. The response: `{"status": "success", "results": []}`. The agent saw success and moved on. We added `partial_success` and `all_failed` as additional status values.

**Dead features in agent-info.** The manifest advertised search modes that existed in code but were never wired into the dispatch path. An agent that called `search --mode deep` got an "unknown mode" error despite agent-info promising it worked. If agent-info says the tool can do something, it must actually do it.

**`--help` returned exit code 3.** We used `try_parse()` and routed all clap errors through the JSON error handler. But `--help` and `--version` aren't errors -- they're informational requests. When an agent ran `tool --help` it got exit code 3 ("bad input") and a suggestion to "check arguments with --help." The agent thought it had made a mistake and tried to fix arguments that were never wrong. The fix: check `e.kind()` for `DisplayHelp` and `DisplayVersion`, wrap the text in a success envelope, and exit 0.

**Inconsistent subcommand names.** Our `inbox` group used `ls` but `account` used `list`. An agent that learned `inbox ls` reasonably tried `account ls` and got an error. Then it tried `account list`, which worked, but the recovery cost tokens and trust. Every CRUD operation should accept both the long form and the short alias (`list`/`ls`, `delete`/`rm`). Use clap's `visible_alias` to make them discoverable. Document aliases in `agent-info` so the agent knows both forms exist.

## Command Naming Conventions

Agents learn patterns from one subcommand group and apply them everywhere. If `inbox` uses `ls` and `account` uses `list`, the agent will fail on one of them every time. Two rules prevent this:

**1. Always alias CRUD subcommands.** Pick one as primary, alias the other with `visible_alias`:

| Operation | Primary | Alias | Attribute |
|-----------|---------|-------|-----------|
| List | `list` | `ls` | `#[command(visible_alias = "ls")]` |
| Create | `create` | `new` | `#[command(visible_alias = "new")]` |
| Delete | `delete` | `rm` | `#[command(visible_alias = "rm")]` |
| Show | `show` | `get` | `#[command(visible_alias = "get")]` |

**2. Be consistent across subcommand groups.** If `inbox list` works, `account list` must also work. Same names, same aliases, same argument patterns. An agent that successfully calls one group will attempt the same syntax on every other group.

Document aliases in `agent-info` using `"list | ls"` format so agents can discover both forms from the manifest.

## What's Inside

```
agent-cli-framework/
  README.md              # You are here
  LICENSE                # MIT
  CONTRIBUTING.md        # How to contribute
  example/
    Cargo.toml           # Dependencies
    src/main.rs          # Complete working example (~280 lines)
```

The example is a `greeter` CLI that demonstrates all five patterns in one file. It's meant to be read, copied, and adapted.

## Production CLIs Using This Architecture

| CLI | What it does | Install |
|-----|-------------|---------|
| [search-cli](https://github.com/199-biotechnologies/search-cli) | 11 search providers, 14 modes, one binary | `cargo install agent-search` |
| [autoresearch](https://github.com/199-biotechnologies/autoresearch-cli) | Autonomous experiment loops for any metric | `cargo install autoresearch` |
| [xmaster](https://github.com/199-biotechnologies/xmaster) | X/Twitter CLI with dual backends | `cargo install xmaster` |
| [email-cli](https://github.com/199-biotechnologies/email-cli) | Agent-friendly email via Resend API | `cargo install email-cli` |

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

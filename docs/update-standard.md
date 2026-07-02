# Update Standard

This is the framework standard for `update`, release artifacts, and package-manager
handoff. The command is agent-facing, so it must be predictable, non-interactive,
machine-readable, and honest about the channel that owns the installed binary.

## Verdict

The old rule, "three install paths, one update mechanism", is wrong.

The standard is:

> One `update` command, distribution-aware update paths.

A binary installed by Homebrew should be upgraded by Homebrew. A binary installed
by Cargo should be upgraded by Cargo or cargo-binstall. A standalone installer
binary may replace itself from signed or attested release artifacts. Managed
environments may disable mutation entirely.

This matches the current high-standard pattern used by mature CLIs such as uv:
self-update is only enabled for standalone installer installs; other install
methods use their package manager's upgrade path.

## Command Contract

Every framework CLI that supports updates exposes:

```bash
<tool> update --check
<tool> update
```

`update --check` is always safe:

- no filesystem mutation
- no shell profile mutation
- no package-manager upgrade
- no stdout outside the normal success envelope
- exit `0` when the check completes, even if an update is available
- exit `1` for transient network/release lookup failures
- exit `2` for invalid update configuration

`update` either applies the update through the correct owner channel, or returns a
tested instruction the agent can run literally. It must never try to overwrite a
Homebrew, Cargo, npm, Bun, uv tool, pipx, apt, winget, or enterprise-managed
binary with a raw GitHub asset.

## Required JSON Shape

`update --check --json` returns a normal success envelope. `data` must include:

```json
{
  "current_version": "1.2.3",
  "latest_version": "1.2.4",
  "status": "update_available",
  "install_source": "homebrew",
  "update_mode": "package_manager",
  "upgrade_command": "brew upgrade your-cli",
  "release_url": "https://github.com/owner/repo/releases/tag/v1.2.4",
  "requires_skill_reinstall": true
}
```

Allowed `status` values:

- `up_to_date`
- `update_available`
- `updated`
- `disabled`
- `managed_install`
- `unsupported_platform`

Allowed `install_source` values:

- `standalone`
- `homebrew`
- `cargo`
- `cargo_binstall`
- `npm`
- `bun`
- `uv_tool`
- `pipx`
- `winget`
- `scoop`
- `apt`
- `managed`
- `unknown`

Allowed `update_mode` values:

- `self_replace`
- `package_manager`
- `instructions_only`
- `disabled`

`agent-info` must describe the command, options, update sources, and package
names. If any field is hand-maintained, tests must prove it matches the CLI.

## Install Source Detection

Detection order:

1. Explicit config override, for example `update.install_source = "homebrew"`.
2. Build-time metadata, for example `ACF_INSTALL_SOURCE=standalone` embedded by
   release CI or installer scripts.
3. Executable path heuristics:
   - Homebrew: path contains `/Cellar/<formula>/` or `/opt/homebrew/bin`.
   - Cargo: path is under `$CARGO_HOME/bin` or `~/.cargo/bin`.
   - npm: path is under an npm global prefix or package shim.
   - Bun package manager: path is under the Bun global bin directory.
   - uv tool: path resolves under the uv tools directory.
   - pipx: path is under `~/.local/bin` and `pipx list` confirms ownership.
4. Package-manager queries when available:
   - `brew list --versions <formula>`
   - `cargo install --list`
   - `cargo binstall --version` plus crate metadata
   - `npm list -g <package> --json`
   - `bun update --global --dry-run <package>`
   - `uv tool list`
   - `pipx list --json`
5. `unknown`, with a safe manual instruction.

Never guess silently. If detection is uncertain, return `install_source:
"unknown"` and `update_mode: "instructions_only"`.

## Channel Rules

### Standalone

Use GitHub Releases or the configured release host.

Requirements:

- select an asset by exact OS, architecture, libc, and binary name
- reject prereleases unless `update.allow_prerelease = true`
- download over HTTPS
- verify SHA256 before replacement
- verify signature or provenance when configured
- unpack to a temp directory on the same filesystem as the current executable
- run `<new-binary> --version` before replacing
- replace atomically where the OS permits it
- leave the old binary untouched if any check fails
- after success, tell the agent to run `<tool> skill install` when the skill is
  embedded in the binary

Recommended implementation options:

- Rust: `self_update` is acceptable for simple GitHub Release replacement, but
  wrap it with install-source detection and checksum/provenance policy.
- Rust with generated release artifacts: prefer cargo-dist for archives,
  installers, Homebrew formulae, checksums, and optional updater support.

### Homebrew

Homebrew owns the file layout and bottle checksums. Do not self-replace.

`update --check` may use `brew outdated --json=v2 <formula>` or release metadata.
`update` should return or run:

```bash
brew upgrade <formula>
```

If the formula is in a tap:

```bash
brew upgrade owner/tap/<formula>
```

Release CI should publish a tap formula or use cargo-dist's Homebrew installer.
The formula must have a stable `homepage`, `url`, `sha256`, `license`, and
working `brew test`.

### Cargo

Cargo-installed tools are source builds. Do not self-replace.

Preferred instruction:

```bash
cargo install --locked --force <crate>
```

If cargo-binstall is available and the project publishes compatible artifacts:

```bash
cargo binstall --no-confirm <crate>
```

Use crates.io metadata to check the latest published stable version. Do not treat
GitHub-only releases as Cargo updates unless the crate was installed from Git.

### uv Tool

uv is a release and install channel for Python CLIs, not a Rust binary
replacement mechanism.

Use it when the CLI is a Python package with console scripts published to PyPI or
another Python index. Release with `uv build` and `uv publish`; install and
upgrade with `uv tool install <package>` and `uv tool upgrade <package>`.

Do not self-replace a uv-installed tool. uv owns the virtual environment, linked
executable, Python version, and upgrade constraints.

Preferred instruction:

```bash
uv tool upgrade <package>
```

If the original install pinned constraints and the requested update should move
beyond them, return:

```bash
uv tool install <package>
```

### Bun

Bun can appear in two different release shapes:

- Bun package-manager install: package with a `bin` entry installed globally by
  Bun. Defer to Bun for upgrades.
- Bun standalone executable: artifact produced by `bun build --compile`. Treat
  this like any other standalone binary and use the standalone artifact rules
  for checksums, provenance, temp-file staging, and atomic replacement.

For a Bun package-manager install, use:

```bash
bun update --global <package>
```

For npm registry publication, `bun publish` is acceptable, but CI must use
`bun publish --dry-run` or `bun pm pack` before release to verify package
contents. For standalone executable release, build each target explicitly with
`bun build --compile --target=...`, then publish the artifacts through the same
release/checksum/provenance path as Rust binaries.

### JavaScript, Python, and Other Languages

The framework is Rust-first, but the update standard is language-neutral:

- If the project ships a standalone executable, use the standalone rules.
- If it is installed through npm, Bun, pipx, uv tool, pip, or another package
  manager, defer to that manager.
- If cargo-dist or another release tool wraps a non-Rust binary, the same
  release artifact, checksum, and channel rules apply.

Examples:

```bash
npm update -g <package>
bun update --global <package>
uv tool upgrade <package>
pipx upgrade <package>
winget upgrade --id <package-id>
scoop update <package>
```

## Release Pipeline Standard

For Rust CLIs, the recommended baseline is:

1. Build and test on every PR:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features -- -D warnings`
   - `cargo test --locked`
   - `cargo dist plan` if cargo-dist is used
2. Publish from signed version tags only:
   - `vX.Y.Z` for single-crate repos
   - `<crate>/vX.Y.Z` or `<crate>-vX.Y.Z` for multi-package repos
3. Generate release artifacts in CI, not on a developer laptop.
4. Produce artifacts for at least:
   - `x86_64-unknown-linux-gnu`
   - `aarch64-unknown-linux-gnu`
   - `x86_64-apple-darwin`
   - `aarch64-apple-darwin`
   - `x86_64-pc-windows-msvc` if Windows is supported
5. Publish:
   - GitHub Release archives
   - SHA256 checksums
   - Homebrew tap formula when supported
   - crates.io package when supported
   - PyPI package when `uv tool` is a supported channel
   - npm package when npm or Bun package-manager installs are supported
   - cargo-binstall-compatible metadata or artifact names when supported
6. Add provenance:
   - GitHub artifact attestations for release artifacts
   - SBOM or auditable dependency metadata for security-sensitive tools
7. Smoke-test installs:
   - standalone installer
   - Homebrew install and upgrade path
   - `cargo install --locked --force <crate>`
   - `cargo binstall --no-confirm <crate>` when supported
   - `uv tool install <package>` and `uv tool upgrade <package>` when supported
   - `bun install --global <package>` and `bun update --global <package>` when supported
   - `<tool> agent-info`
   - `<tool> update --check --json`

## Developer Configuration

Every project must set these consciously:

```toml
[update]
enabled = true
install_source = "auto"
owner = "your-org"
repo = "your-repo"
crate_name = "your-cli"
formula = "your-cli"
tap = "your-org/tap"

# Standalone-channel policy -- implement these when you ship a standalone
# installer that self-replaces:
allow_prerelease = false
require_checksum = true
require_attestation = false
```

Key names match the `example/` config struct (`formula`, `tap`) -- do not
invent variants like `brew_formula`.

For enterprise or managed environments:

```toml
[update]
enabled = false
install_source = "managed"
```

The error suggestion for disabled updates must point to the exact manager-owned
command or internal rollout process, never a generic "download latest" message.

## Tests

Minimum tests:

- `update --check --json` returns a valid success envelope.
- `update --check` writes no raw text to stdout when piped.
- disabled update returns `status: "disabled"` and exit `0`.
- Homebrew channel returns `brew upgrade ...` and does not call `self_update`.
- Cargo channel returns `cargo install --locked --force ...` or `cargo binstall
  --no-confirm ...` and does not call `self_update`.
- unknown channel returns `instructions_only`, not a blind self-replacement.
- malformed update config exits `2`.
- release lookup failures exit `1`.
- `agent-info` documents every update option that exists in clap.
- suggestions are exact commands that pass a shell-parse test.

## References

- uv installation and upgrade policy: https://github.com/astral-sh/uv/blob/main/docs/getting-started/installation.md
- cargo-dist Homebrew installer: https://axodotdev.github.io/cargo-dist/book/installers/homebrew.html
- cargo-dist updater config: https://axodotdev.github.io/cargo-dist/book/reference/config.html#install-updater
- cargo-dist supply-chain security: https://axodotdev.github.io/cargo-dist/book/supplychain-security/
- cargo-binstall: https://github.com/cargo-bins/cargo-binstall
- Cargo install: https://doc.rust-lang.org/stable/cargo/commands/cargo-install.html
- Homebrew formula cookbook: https://docs.brew.sh/Formula-Cookbook
- GitHub artifact attestations: https://docs.github.com/actions/concepts/security/artifact-attestations
- uv tool/publish CLI reference: https://docs.astral.sh/uv/reference/cli/
- Bun standalone executables: https://bun.sh/docs/bundler/executables
- Bun global install: https://bun.sh/docs/pm/cli/install
- Bun global update: https://bun.sh/docs/pm/cli/update
- Bun publish: https://bun.sh/docs/pm/cli/publish

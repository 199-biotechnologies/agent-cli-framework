# Contributing to Agent CLI Framework

Thanks for your interest in contributing.

## How to contribute

1. Fork the repo and create a branch from `main`.
2. Make your changes. Keep them focused -- one concern per PR.
3. If you add a pattern, include it in both the README and the `example/` CLI.
4. Run the full test suite:
   ```bash
   cd example && cargo test --locked
   ```
   All integration tests must pass. They verify:
   - Exit code contracts (0-4)
   - JSON envelope structure (success + error)
   - `--help`/`--version` exit 0
   - `agent-info` manifest matches actual commands
   - Piped output auto-switches to JSON
5. Open a pull request with a clear description of what you changed and why.

## Project structure

```
README.md              # Full framework documentation: philosophy, patterns, reusable modules
AGENTS.md              # Condensed build instructions for AI coding agents
CONTRIBUTING.md        # This file
.github/workflows/     # CI: builds and tests on macOS + Linux
example/
  src/
    main.rs            # Entry point: parse, detect format, dispatch, exit
    cli.rs             # Clap derive definitions (--json, --quiet, all commands)
    config.rs          # Config loading via figment (defaults -> TOML -> env vars)
    error.rs           # Error enum with exit_code(), error_code(), suggestion()
    output.rs          # Format detection, Ctx struct, JSON envelope helpers
    commands/
      mod.rs           # Re-exports
      hello.rs         # Domain command example
      agent_info.rs    # Enriched capability manifest with arg schemas
      config.rs        # config show/path
      contract.rs      # Hidden deterministic exit-code trigger for tests
      skill.rs         # Skill install + status (uses CARGO_PKG_NAME)
      update.rs        # Self-update (repo configurable via config)
  tests/
    exit_code_contracts.rs   # All 5 exit codes verified
    output_contracts.rs      # JSON envelope shape, quiet flag, help wrapping
    agent_info_contract.rs   # Manifest fields, routable commands, arg schemas
  Cargo.toml
```

## What's useful

- New patterns or refinements to existing ones, backed by real-world agent usage.
- Bug fixes or improvements to the example CLI.
- Documentation improvements that make the patterns clearer or more precise.
- Additional integration tests verifying framework invariants.
- Links to additional CLIs built with this architecture.

## Guidelines

- The example demonstrates the five core patterns plus the entry point, error type, and output helpers. Reusable modules like config loading, secret handling, and HTTP retry are documented as code patterns in the README -- they don't need to be in the example.
- Keep the example minimal -- it demonstrates patterns, not a real product.
- Ensure the README, AGENTS.md, and example stay consistent with each other.
- All new patterns must have corresponding integration tests.

## Style

- Write like you're explaining to a colleague. Short sentences. Active voice.
- Code examples should be minimal and runnable.
- If you reference a claim, link to the source.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

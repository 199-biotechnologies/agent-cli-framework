# Contributing to Agent CLI Framework

Thanks for your interest in contributing.

## How to contribute

1. Fork the repo and create a branch from `main`.
2. Make your changes. Keep them focused -- one concern per PR.
3. If you add a pattern, include it in both the README and the `example/` CLI.
4. Test that the example builds and all behaviors work:
   ```bash
   cd example && cargo build --release
   ./target/release/greeter hello Boris --style pirate
   ./target/release/greeter hello Boris | jq        # JSON envelope
   ./target/release/greeter hello "" 2>&1; echo $?  # exit code 3
   ./target/release/greeter --help > /dev/null; echo $?  # exit code 0
   ./target/release/greeter agent-info | jq          # valid manifest
   ```
5. Open a pull request with a clear description of what you changed and why.

## Project structure

```
README.md              # Full framework documentation: philosophy, patterns, reusable modules
AGENTS.md              # Condensed build instructions for AI coding agents
CONTRIBUTING.md        # This file
example/
  src/
    main.rs            # Entry point: parse, detect format, dispatch, exit
    cli.rs             # Clap derive definitions
    error.rs           # Error enum with exit_code(), error_code(), suggestion()
    output.rs          # Format detection + JSON envelope helpers
    commands/
      mod.rs           # Re-exports
      hello.rs         # Domain command example
      agent_info.rs    # Capability manifest
      skill.rs         # Skill install + status
      update.rs        # Self-update
  Cargo.toml
```

## What's useful

- New patterns or refinements to existing ones, backed by real-world agent usage.
- Bug fixes or improvements to the example CLI.
- Documentation improvements that make the patterns clearer or more precise.
- Integration tests that verify framework invariants (help exits 0, piped output is valid JSON, etc.).
- Links to additional CLIs built with this architecture.

## Guidelines

- The example must demonstrate every pattern described in the README. If the README shows it, the example should use it.
- Keep the example minimal -- it demonstrates patterns, not a real product.
- Ensure the README, AGENTS.md, and example stay consistent with each other.

## Style

- Write like you're explaining to a colleague. Short sentences. Active voice.
- Code examples should be minimal and runnable.
- If you reference a claim, link to the source.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

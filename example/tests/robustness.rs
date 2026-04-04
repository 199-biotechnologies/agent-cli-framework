//! Robustness tests: verify recovery from bad state.
//!
//! These tests ensure discovery and diagnostic commands work even when
//! configuration is malformed, and that enforced constraints match agent-info.

use assert_cmd::Command;

fn greeter() -> Command {
    Command::cargo_bin("greeter").unwrap()
}

// ── Malformed config resilience ────────────────────────────────────────────

/// agent-info must work even with a broken config file.
#[test]
fn agent_info_works_with_malformed_config() {
    let tmp = tempfile::tempdir().unwrap();
    // Write an invalid TOML file where figment will look.
    let config_dir = tmp.path().join("Library/Application Support/greeter");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "{{invalid toml").unwrap();

    greeter()
        .env("HOME", tmp.path())
        .arg("agent-info")
        .assert()
        .code(0);
}

/// config path must work even with a broken config file.
#[test]
fn config_path_works_with_malformed_config() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("Library/Application Support/greeter");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "{{invalid toml").unwrap();

    greeter()
        .env("HOME", tmp.path())
        .args(["config", "path"])
        .assert()
        .code(0);
}

/// config show should fail gracefully with exit 2 on malformed config.
#[test]
fn config_show_fails_with_malformed_config() {
    let tmp = tempfile::tempdir().unwrap();
    let config_dir = tmp.path().join("Library/Application Support/greeter");
    std::fs::create_dir_all(&config_dir).unwrap();
    std::fs::write(config_dir.join("config.toml"), "{{invalid toml").unwrap();

    greeter()
        .env("HOME", tmp.path())
        .args(["config", "show"])
        .assert()
        .code(2);
}

// ── Constraint enforcement ─────────────────────────────────────────────────

/// Invalid --style value should be rejected by clap (exit 3).
#[test]
fn invalid_style_rejected() {
    greeter()
        .args(["hello", "World", "--style", "nonsense"])
        .assert()
        .code(3);
}

/// hello command works without --quiet even when HOME is unusual.
#[test]
fn hello_works_with_temp_home() {
    let tmp = tempfile::tempdir().unwrap();
    greeter()
        .env("HOME", tmp.path())
        .args(["hello", "World"])
        .assert()
        .code(0);
}

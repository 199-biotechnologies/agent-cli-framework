//! Verify the semantic exit-code contract (0-4).
//!
//! Uses the hidden `contract` command for deterministic triggers and
//! real commands for natural exit-code coverage.

use assert_cmd::Command;

fn greeter() -> Command {
    Command::cargo_bin("greeter").unwrap()
}

// ── Contract command: deterministic 0-4 ────────────────────────────────────

#[test]
fn contract_exit_0() {
    greeter().args(["contract", "0"]).assert().code(0);
}

#[test]
fn contract_exit_1_transient() {
    greeter().args(["contract", "1"]).assert().code(1);
}

#[test]
fn contract_exit_2_config() {
    greeter().args(["contract", "2"]).assert().code(2);
}

#[test]
fn contract_exit_3_bad_input() {
    greeter().args(["contract", "3"]).assert().code(3);
}

#[test]
fn contract_exit_4_rate_limited() {
    greeter().args(["contract", "4"]).assert().code(4);
}

// ── Real commands: natural exit codes ──────────────────────────────────────

#[test]
fn hello_success_exits_0() {
    greeter().args(["hello", "World"]).assert().code(0);
}

#[test]
fn help_exits_0() {
    greeter().arg("--help").assert().code(0);
}

#[test]
fn version_exits_0() {
    greeter().arg("--version").assert().code(0);
}

#[test]
fn agent_info_exits_0() {
    greeter().arg("agent-info").assert().code(0);
}

#[test]
fn config_path_exits_0() {
    greeter().args(["config", "path"]).assert().code(0);
}

#[test]
fn config_show_exits_0() {
    greeter().args(["config", "show"]).assert().code(0);
}

#[test]
fn missing_subcommand_exits_3() {
    // No subcommand at all is a parse error.
    greeter().assert().code(3);
}

#[test]
fn hello_missing_name_exits_3() {
    // `hello` requires a positional <name>.
    greeter().arg("hello").assert().code(3);
}

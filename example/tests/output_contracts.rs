//! Verify JSON envelope structure on stdout/stderr.
//!
//! assert_cmd runs the binary in a pipe (not a TTY), so JSON auto-detection
//! kicks in — no need for `--json`.

use assert_cmd::Command;

fn greeter() -> Command {
    Command::cargo_bin("greeter").unwrap()
}

// ── Success envelope ───────────────────────────────────────────────────────

#[test]
fn success_envelope_shape() {
    let out = greeter()
        .args(["hello", "World"])
        .output()
        .unwrap();

    assert!(out.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout should be valid JSON");

    assert_eq!(json["version"], "1");
    assert_eq!(json["status"], "success");
    assert!(json["data"].is_object(), "envelope must have a 'data' field");
    assert_eq!(json["data"]["name"], "World");
}

#[test]
fn success_envelope_with_json_flag() {
    let out = greeter()
        .args(["hello", "World", "--json"])
        .output()
        .unwrap();

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("--json stdout should be valid JSON");

    assert_eq!(json["status"], "success");
    assert_eq!(json["data"]["style"], "friendly");
}

// ── Error envelope ─────────────────────────────────────────────────────────

#[test]
fn error_envelope_shape() {
    let out = greeter()
        .args(["contract", "3"])
        .output()
        .unwrap();

    assert!(!out.status.success());

    // Error envelopes go to stderr.
    let json: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("stderr should be valid JSON");

    assert_eq!(json["version"], "1");
    assert_eq!(json["status"], "error");
    assert!(json["error"].is_object(), "error envelope must have 'error' field");
    assert!(json["error"]["code"].is_string(), "error must have 'code'");
    assert!(json["error"]["message"].is_string(), "error must have 'message'");
    assert!(json["error"]["suggestion"].is_string(), "error must have 'suggestion'");
}

#[test]
fn error_code_matches_variant() {
    // Exit 2 = config_error
    let out = greeter().args(["contract", "2"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "config_error");

    // Exit 4 = rate_limited
    let out = greeter().args(["contract", "4"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "rate_limited");
}

// ── Help/version wrapping ──────────────────────────────────────────────────

#[test]
fn help_wrapped_in_envelope_when_piped() {
    let out = greeter().arg("--help").output().unwrap();
    assert!(out.status.success());

    // When piped, --help should be wrapped in a success envelope.
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("piped --help should be JSON");

    assert_eq!(json["status"], "success");
    assert!(json["data"]["usage"].is_string());
}

#[test]
fn version_wrapped_in_envelope_when_piped() {
    let out = greeter().arg("--version").output().unwrap();
    assert!(out.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("piped --version should be JSON");

    assert_eq!(json["status"], "success");
}

// ── Quiet flag ─────────────────────────────────────────────────────────────

#[test]
fn quiet_still_emits_json() {
    // --quiet should not suppress JSON output (agents need the envelope).
    let out = greeter()
        .args(["hello", "World", "--json", "--quiet"])
        .output()
        .unwrap();

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("--quiet should not suppress JSON");

    assert_eq!(json["status"], "success");
}

// ── Parse error envelope ───────────────────────────────────────────────────

#[test]
fn parse_error_wrapped_in_envelope() {
    let out = greeter()
        .arg("hello") // missing required <name>
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(3));

    let json: serde_json::Value =
        serde_json::from_slice(&out.stderr).expect("parse error should be JSON on stderr");

    assert_eq!(json["status"], "error");
    assert_eq!(json["error"]["code"], "invalid_input");
    assert!(json["error"]["suggestion"]
        .as_str()
        .unwrap()
        .contains("--help"));
}

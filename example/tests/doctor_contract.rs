//! Verify the doctor contract: structured pass/warn/fail checks,
//! exit 0 when nothing fails, exit 2 when any check fails.

mod common;
use common::{greeter_in, write_config_in};

#[test]
fn doctor_passes_out_of_the_box() {
    // Fresh HOME, no config file: missing config is a warning, not a failure.
    let tmp = tempfile::tempdir().unwrap();
    let out = greeter_in(tmp.path())
        .args(["--json", "doctor"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("doctor should emit JSON");
    assert_eq!(json["status"], "success");
    assert!(json["data"]["checks"].as_array().unwrap().len() >= 2);
    assert_eq!(json["data"]["summary"]["fail"], 0);
}

#[test]
fn doctor_fails_on_malformed_config() {
    let tmp = tempfile::tempdir().unwrap();
    write_config_in(tmp.path(), "style = [not valid toml");

    let out = greeter_in(tmp.path())
        .args(["--json", "doctor"])
        .output()
        .unwrap();

    assert_eq!(out.status.code(), Some(2));

    // Report still lands on stdout; the error envelope goes to stderr.
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(report["data"]["summary"]["fail"].as_u64().unwrap() >= 1);

    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["status"], "error");
    assert_eq!(err["error"]["code"], "config_error");
}

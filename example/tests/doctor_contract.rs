//! Verify the doctor contract: structured pass/warn/fail checks,
//! exit 0 when nothing fails, exit 2 when any check fails.

use assert_cmd::Command;
use std::path::{Path, PathBuf};

fn greeter() -> Command {
    Command::cargo_bin("greeter").unwrap()
}

fn config_path_for_home(home: &Path) -> PathBuf {
    let out = greeter()
        .env("HOME", home)
        .args(["--json", "config", "path"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("config path should be JSON");
    PathBuf::from(json["data"]["path"].as_str().unwrap())
}

#[test]
fn doctor_passes_out_of_the_box() {
    // Fresh HOME, no config file: missing config is a warning, not a failure.
    let tmp = tempfile::tempdir().unwrap();
    let out = greeter()
        .env("HOME", tmp.path())
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
    let path = config_path_for_home(tmp.path());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "style = [not valid toml").unwrap();

    let out = greeter()
        .env("HOME", tmp.path())
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

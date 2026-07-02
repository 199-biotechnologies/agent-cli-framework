//! Shared test helpers: run the binary against a fully isolated home.
//!
//! Overriding HOME alone is NOT enough. GitHub's Linux runners export
//! XDG_CONFIG_HOME, which the `directories` crate prefers over HOME, so
//! tests that only set HOME silently read and write the runner's real
//! config directory -- polluting every other test. Point HOME and all
//! XDG dirs at the temp home so the child process is hermetic on every
//! platform.

#![allow(dead_code)] // not every test file uses every helper

use assert_cmd::Command;
use std::path::{Path, PathBuf};

/// The greeter binary with HOME and XDG dirs isolated to `home`.
pub fn greeter_in(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("greeter").unwrap();
    cmd.env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("XDG_DATA_HOME", home.join(".local/share"))
        .env("XDG_CACHE_HOME", home.join(".cache"));
    cmd
}

/// Ask the binary where its config file lives for this home. Discovery via
/// `config path` keeps tests platform-correct (macOS uses
/// Library/Application Support, Linux uses ~/.config) instead of hardcoding
/// one platform's layout.
pub fn config_path_in(home: &Path) -> PathBuf {
    let out = greeter_in(home)
        .args(["--json", "config", "path"])
        .output()
        .unwrap();
    assert!(out.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("config path should be JSON");
    PathBuf::from(json["data"]["path"].as_str().unwrap())
}

/// Write a config file into an isolated home, creating parent dirs.
pub fn write_config_in(home: &Path, contents: &str) -> PathBuf {
    let path = config_path_in(home);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, contents).unwrap();
    path
}

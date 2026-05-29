//! AC6 (MUST): An invalid `--at` (e.g. `25:00`) or invalid `--tz` exits non-zero
//! without writing the store.

use std::process::Command;
use tempfile::TempDir;

fn bin() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.pop();
    p.push("wm-almanac");
    p
}

#[test]
fn ac6_invalid_at_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "bad time",
            "--at",
            "25:00",
            "--every",
            "daily",
            "--say",
            "x",
        ])
        .status()
        .unwrap();
    assert!(!status.success(), "invalid --at should exit non-zero");

    // Store should not have been created
    let store_path = dir.path().join("wm-almanac/schedule.toml");
    assert!(
        !store_path.exists(),
        "schedule.toml should NOT be created on invalid input"
    );
}

#[test]
fn ac6_invalid_tz_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "bad tz",
            "--at",
            "10:00",
            "--every",
            "daily",
            "--say",
            "x",
            "--tz",
            "NotATimezone/Invalid",
        ])
        .status()
        .unwrap();
    assert!(!status.success(), "invalid --tz should exit non-zero");

    let store_path = dir.path().join("wm-almanac/schedule.toml");
    assert!(
        !store_path.exists(),
        "schedule.toml should NOT be created on invalid tz"
    );
}

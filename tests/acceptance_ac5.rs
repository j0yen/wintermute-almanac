//! AC5 (MUST): `--every mon,wed,fri` parses to `Weekly{days:[Mon,Wed,Fri]}`;
//! `--every once:2026-06-01` parses to `Once`; invalid `--every` exits non-zero
//! with a clear message.

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
fn ac5_weekly_parses_correctly() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "three day week",
            "--at",
            "10:00",
            "--every",
            "mon,wed,fri",
            "--say",
            "go",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    let arr: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&out.stdout).unwrap()).unwrap();
    let rec = &arr[0]["recurrence"];
    assert_eq!(rec["kind"], "weekly", "recurrence should be weekly");
    let days = rec["days"].as_array().unwrap();
    assert_eq!(days.len(), 3, "should have 3 days");
}

#[test]
fn ac5_once_parses_correctly() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "one time",
            "--at",
            "14:00",
            "--every",
            "once:2026-06-01",
            "--say",
            "once",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    let arr: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&out.stdout).unwrap()).unwrap();
    let rec = &arr[0]["recurrence"];
    assert_eq!(rec["kind"], "once", "recurrence should be once");
    assert_eq!(rec["date"], "2026-06-01");
}

#[test]
fn ac5_invalid_every_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "bad",
            "--at",
            "10:00",
            "--every",
            "notaday,xyz",
            "--say",
            "x",
        ])
        .status()
        .unwrap();
    assert!(!status.success(), "invalid --every should exit non-zero");
}

//! AC7 (MUST): `wm-almanac next --format json` computes the next fire instant for
//! the soonest enabled entry in its IANA tz (DST-correct via chrono-tz) and prints
//! `{id, fire_ts_unix, label}`; with an empty/all-disabled store it exits 0
//! emitting `null`/empty and a "no entries due" note.

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
fn ac7_next_json_empty_store() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["next", "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "next on empty store should exit 0");
    let stdout = std::str::from_utf8(&out.stdout).unwrap().trim();
    assert_eq!(stdout, "null", "empty store should output null");
}

#[test]
fn ac7_next_text_empty_store() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["next"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = std::str::from_utf8(&out.stdout).unwrap();
    assert!(stdout.contains("no entries due"), "should print 'no entries due'");
}

#[test]
fn ac7_next_json_with_entry() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    // Add a daily entry
    Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .env("WM_TIMEZONE", "UTC")
        .args([
            "add",
            "--label",
            "daily check",
            "--at",
            "23:59",
            "--every",
            "daily",
            "--say",
            "check",
            "--tz",
            "UTC",
        ])
        .status()
        .unwrap();

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["next", "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = std::str::from_utf8(&out.stdout).unwrap().trim();
    // Should not be "null"
    assert_ne!(stdout, "null", "should have a next entry");
    let val: serde_json::Value = serde_json::from_str(stdout).unwrap();
    assert!(val.get("id").is_some(), "id field missing");
    assert!(val.get("fire_ts_unix").is_some(), "fire_ts_unix field missing");
    assert!(val.get("label").is_some(), "label field missing");
    assert!(val["fire_ts_unix"].as_i64().unwrap() > 0, "fire_ts_unix should be a positive timestamp");
}

#[test]
fn ac7_next_all_disabled_returns_null() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    // Add an entry then disable it
    Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "disabled entry",
            "--at",
            "12:00",
            "--every",
            "daily",
            "--say",
            "x",
            "--tz",
            "UTC",
        ])
        .status()
        .unwrap();

    // Get the ID
    let list_out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    let arr: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&list_out.stdout).unwrap()).unwrap();
    let id = arr[0]["id"].as_str().unwrap().to_owned();

    Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["disable", &id])
        .status()
        .unwrap();

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["next", "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = std::str::from_utf8(&out.stdout).unwrap().trim();
    assert_eq!(stdout, "null", "all-disabled store should return null");
}

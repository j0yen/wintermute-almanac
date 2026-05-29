//! AC2 (MUST): `wm-almanac list --format json` returns a JSON array containing
//! the entry with all fields; `--format text` prints a human line per entry.
//! Default is text.

use std::process::Command;
use tempfile::TempDir;

fn bin() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.pop();
    p.push("wm-almanac");
    p
}

fn add_entry(data_home: &str) {
    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "test entry",
            "--at",
            "09:00",
            "--every",
            "daily",
            "--say",
            "hello",
            "--category",
            "meal",
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
fn ac2_list_json_contains_entry() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();
    add_entry(data_home);

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list", "--format", "json"])
        .output()
        .expect("failed to run wm-almanac list --format json");

    assert!(out.status.success());
    let json_str = std::str::from_utf8(&out.stdout).unwrap();
    let arr: serde_json::Value = serde_json::from_str(json_str).expect("output should be valid JSON");
    assert!(arr.is_array(), "JSON output should be an array");
    let entries = arr.as_array().unwrap();
    assert!(!entries.is_empty(), "array should have at least one entry");
    let first = &entries[0];
    // Verify required fields are present
    assert!(first.get("id").is_some(), "id field missing");
    assert!(first.get("label").is_some(), "label field missing");
    assert!(first.get("say").is_some(), "say field missing");
    assert!(first.get("recurrence").is_some(), "recurrence field missing");
    assert!(first.get("local_time").is_some(), "local_time field missing");
    assert!(first.get("tz").is_some(), "tz field missing");
    assert!(first.get("category").is_some(), "category field missing");
    assert!(first.get("opt_in").is_some(), "opt_in field missing");
    assert!(first.get("snooze_min").is_some(), "snooze_min field missing");
    assert!(first.get("max_snoozes").is_some(), "max_snoozes field missing");
    assert!(first.get("created_ms").is_some(), "created_ms field missing");
    assert_eq!(first["label"], "test entry");
}

#[test]
fn ac2_list_text_default() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();
    add_entry(data_home);

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list"])
        .output()
        .expect("failed to run wm-almanac list");

    assert!(out.status.success());
    let text = std::str::from_utf8(&out.stdout).unwrap();
    assert!(text.contains("test entry"), "text output should contain entry label");
}

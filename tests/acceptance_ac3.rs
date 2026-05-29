//! AC3 (MUST): `wm-almanac remove <id>` deletes only that entry; `list` no longer
//! shows it; other entries untouched.

use std::process::Command;
use tempfile::TempDir;

fn bin() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.pop();
    p.push("wm-almanac");
    p
}

fn add_and_get_id(data_home: &str, label: &str) -> String {
    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            label,
            "--at",
            "10:00",
            "--every",
            "daily",
            "--say",
            "test",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());

    // list --format json to get the ID
    let list_out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    let arr: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&list_out.stdout).unwrap()).unwrap();
    // Find the entry with matching label
    arr.as_array()
        .unwrap()
        .iter()
        .find(|e| e["label"] == label)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn ac3_remove_deletes_only_target() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let id1 = add_and_get_id(data_home, "entry one");
    let _id2 = add_and_get_id(data_home, "entry two");

    // Remove entry one
    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["remove", &id1])
        .status()
        .unwrap();
    assert!(status.success(), "remove should exit 0");

    // list should not contain entry one
    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    let arr: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&out.stdout).unwrap()).unwrap();
    let entries = arr.as_array().unwrap();
    assert_eq!(entries.len(), 1, "only one entry should remain");
    assert_eq!(entries[0]["label"], "entry two", "entry two should remain");
}

#[test]
fn ac3_remove_nonexistent_exits_nonzero() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["remove", "nonexistent-id"])
        .status()
        .unwrap();
    assert!(!status.success(), "removing non-existent id should fail");
}

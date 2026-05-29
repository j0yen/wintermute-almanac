//! AC8 (MUST): Store writes are atomic (temp-file + rename); a load against a
//! missing file yields an empty schedule, not an error.

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
fn ac8_list_missing_store_is_empty() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    // No add — store file does not exist yet.
    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    assert!(out.status.success(), "list on missing store should exit 0");
    let json_str = std::str::from_utf8(&out.stdout).unwrap();
    let arr: serde_json::Value = serde_json::from_str(json_str).unwrap();
    assert!(arr.is_array());
    assert!(arr.as_array().unwrap().is_empty(), "empty store should return []");
}

#[test]
fn ac8_no_leftover_tmp_files_after_add() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "atomic test",
            "--at",
            "06:00",
            "--every",
            "daily",
            "--say",
            "x",
        ])
        .status()
        .unwrap();

    let store_dir = dir.path().join("wm-almanac");
    let entries: Vec<_> = std::fs::read_dir(&store_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();

    // Only schedule.toml should remain; no .wm-almanac-tmp-* files.
    let tmp_files: Vec<_> = entries
        .iter()
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with(".wm-almanac-tmp-"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        tmp_files.is_empty(),
        "no temp files should remain after atomic write; found: {tmp_files:?}"
    );
}

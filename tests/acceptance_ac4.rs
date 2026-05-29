//! AC4 (MUST): `wm-almanac disable <id>` sets `opt_in=false` (entry retained,
//! shown as disabled in `list`); `enable <id>` restores it.

use std::process::Command;
use tempfile::TempDir;

fn bin() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop();
    p.pop();
    p.push("wm-almanac");
    p
}

fn add_and_get_id(data_home: &str) -> String {
    Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "toggle test",
            "--at",
            "07:30",
            "--every",
            "daily",
            "--say",
            "wake up",
        ])
        .status()
        .unwrap();
    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    let arr: serde_json::Value =
        serde_json::from_str(std::str::from_utf8(&out.stdout).unwrap()).unwrap();
    arr.as_array().unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

#[test]
fn ac4_disable_sets_opt_in_false() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();
    let id = add_and_get_id(data_home);

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["disable", &id])
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
    let entry = &arr.as_array().unwrap()[0];
    assert_eq!(entry["opt_in"], false, "opt_in should be false after disable");
}

#[test]
fn ac4_enable_restores_opt_in() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();
    let id = add_and_get_id(data_home);

    // Disable then re-enable
    Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["disable", &id])
        .status()
        .unwrap();

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["enable", &id])
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
    let entry = &arr.as_array().unwrap()[0];
    assert_eq!(entry["opt_in"], true, "opt_in should be true after enable");

    // Entry still present in list text as "enabled"
    let text_out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["list"])
        .output()
        .unwrap();
    let text = std::str::from_utf8(&text_out.stdout).unwrap();
    assert!(text.contains("enabled"), "text list should show enabled status");
}

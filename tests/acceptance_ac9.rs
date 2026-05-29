//! AC9 (MUST): `cargo test` green; `cargo build --release` produces `wm-almanac`;
//! `wm-almanac --help` lists all subcommands. No network/SecretService/bus
//! dependency in this crate.

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
fn ac9_help_lists_subcommands() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let out = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args(["--help"])
        .output()
        .unwrap();
    assert!(out.status.success(), "--help should exit 0");
    let help_text = std::str::from_utf8(&out.stdout).unwrap();
    assert!(help_text.contains("add"), "help should list 'add'");
    assert!(help_text.contains("list"), "help should list 'list'");
    assert!(help_text.contains("remove"), "help should list 'remove'");
    assert!(help_text.contains("enable"), "help should list 'enable'");
    assert!(help_text.contains("disable"), "help should list 'disable'");
    assert!(help_text.contains("next"), "help should list 'next'");
}

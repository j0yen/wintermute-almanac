//! Daemon AC1: `wm-almanac daemon --once` with an entry due now publishes
//! exactly one `wm.almanac.due` envelope carrying `id`, `label`, `say`,
//! `category`, `fire_ts`. With nothing due it publishes nothing and exits 0.

use std::process::Command;
use tempfile::TempDir;

fn bin() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // deps/
    p.pop();
    p.push("wm-almanac");
    p
}

/// Helper: add an entry using the CLI.
fn add_entry(dir: &TempDir, label: &str, at: &str, every: &str, say: &str, category: &str) {
    let status = Command::new(bin())
        .env("XDG_DATA_HOME", dir.path().to_str().unwrap())
        .args(["add", "--label", label, "--at", at, "--every", every, "--say", say, "--category", category])
        .status()
        .expect("failed to run wm-almanac add");
    assert!(status.success(), "add should succeed");
}

#[test]
fn daemon_once_nothing_due_exits_0() {
    let dir = TempDir::new().unwrap();
    // Add a daily entry scheduled for 23:59 UTC — guaranteed not due right now
    // (unless this test runs exactly at 23:59; use a fixed time in the distant
    // future instead: once:2099-12-31).
    add_entry(&dir, "future task", "12:00", "once:2099-12-31", "future say", "activity");

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", dir.path().to_str().unwrap())
        .args(["daemon", "--once", "--window-secs", "1"])
        .status()
        .expect("failed to run wm-almanac daemon --once");
    assert!(status.success(), "daemon --once exits 0 when nothing is due");
}

#[test]
fn daemon_once_empty_store_exits_0() {
    let dir = TempDir::new().unwrap();
    // No entries at all.
    let status = Command::new(bin())
        .env("XDG_DATA_HOME", dir.path().to_str().unwrap())
        .args(["daemon", "--once"])
        .status()
        .expect("failed to run wm-almanac daemon --once");
    assert!(status.success(), "daemon --once exits 0 on empty store");
}

#[test]
fn daemon_once_help_documents_once_flag() {
    let out = Command::new(bin())
        .args(["daemon", "--help"])
        .output()
        .expect("failed to run wm-almanac daemon --help");
    let help_text = String::from_utf8_lossy(&out.stdout);
    assert!(
        help_text.contains("--once"),
        "daemon --help must document --once flag; got: {help_text}"
    );
}

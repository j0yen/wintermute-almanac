//! AC1 (MUST): `wm-almanac add --label "morning pills" --at 08:00 --every daily
//! --say "time for your blue pill" --category med` exits 0 and persists one
//! entry to `schedule.toml`.

use std::process::Command;
use tempfile::TempDir;

fn bin() -> std::path::PathBuf {
    let mut p = std::env::current_exe().unwrap();
    p.pop(); // deps/
    p.pop(); // test binary dir
    p.push("wm-almanac");
    p
}

#[test]
fn ac1_add_persists_entry() {
    let dir = TempDir::new().unwrap();
    let data_home = dir.path().to_str().unwrap();

    let status = Command::new(bin())
        .env("XDG_DATA_HOME", data_home)
        .args([
            "add",
            "--label",
            "morning pills",
            "--at",
            "08:00",
            "--every",
            "daily",
            "--say",
            "time for your blue pill",
            "--category",
            "med",
        ])
        .status()
        .expect("failed to run wm-almanac");

    assert!(status.success(), "add command should exit 0");

    let store_path = dir.path().join("wm-almanac/schedule.toml");
    assert!(store_path.exists(), "schedule.toml should be created");
    let contents = std::fs::read_to_string(&store_path).unwrap();
    assert!(
        contents.contains("morning pills"),
        "schedule.toml should contain the entry label"
    );
}

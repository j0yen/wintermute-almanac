//! Persistent TOML store for schedule entries.
//!
//! The store lives at `$XDG_DATA_HOME/wm-almanac/schedule.toml`, falling back
//! to `~/.local/share/wm-almanac/schedule.toml`. Writes are atomic via a
//! temp-file + rename pattern. Loading a missing file yields an empty schedule.

use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::entry::Entry;

/// The on-disk TOML shape.
#[derive(Debug, Default, Serialize, Deserialize)]
struct ScheduleFile {
    entries: Vec<Entry>,
}

/// Persistent store wrapping the TOML file.
pub struct Store {
    path: PathBuf,
    schedule: ScheduleFile,
}

impl Store {
    /// Open the store at the default XDG location, creating the directory if needed.
    ///
    /// # Errors
    /// Returns an error if the directory cannot be created or the file cannot be read.
    pub fn open() -> Result<Self> {
        let path = default_store_path()?;
        Self::open_at(path)
    }

    /// Open the store at a specific path (used in tests).
    ///
    /// # Errors
    /// Returns an error if the parent directory cannot be created or the file
    /// cannot be read/parsed.
    pub fn open_at(path: PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating store dir {}", parent.display()))?;
        }
        let schedule = load_schedule(&path)?;
        Ok(Self { path, schedule })
    }

    /// Return all entries (owned clones).
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.schedule.entries
    }

    /// Add an entry and persist.
    ///
    /// # Errors
    /// Returns an error if the store cannot be written.
    pub fn add(&mut self, entry: Entry) -> Result<()> {
        self.schedule.entries.push(entry);
        self.flush()
    }

    /// Remove an entry by ID. Returns `true` if an entry was removed.
    ///
    /// # Errors
    /// Returns an error if the store cannot be written.
    pub fn remove(&mut self, id: &str) -> Result<bool> {
        let before = self.schedule.entries.len();
        self.schedule.entries.retain(|e| e.id != id);
        let removed = self.schedule.entries.len() < before;
        if removed {
            self.flush()?;
        }
        Ok(removed)
    }

    /// Set `opt_in` for an entry. Returns `true` if the entry was found.
    ///
    /// # Errors
    /// Returns an error if the store cannot be written.
    pub fn set_opt_in(&mut self, id: &str, opt_in: bool) -> Result<bool> {
        let entry = self.schedule.entries.iter_mut().find(|e| e.id == id);
        if let Some(e) = entry {
            e.opt_in = opt_in;
            self.flush()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Atomically persist the current schedule to disk.
    ///
    /// Writes to a temp file in the same directory then renames over the target.
    ///
    /// # Errors
    /// Returns an error if serialisation, temp-file creation, or rename fails.
    fn flush(&self) -> Result<()> {
        let serialized =
            toml::to_string_pretty(&self.schedule).context("serialising schedule to TOML")?;
        atomic_write(&self.path, serialized.as_bytes())
            .with_context(|| format!("writing store to {}", self.path.display()))
    }
}

fn default_store_path() -> Result<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let home = std::env::var_os("HOME").unwrap_or_else(|| "/tmp".into());
            PathBuf::from(home).join(".local/share")
        });
    Ok(base.join("wm-almanac/schedule.toml"))
}

fn load_schedule(path: &Path) -> Result<ScheduleFile> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            toml::from_str::<ScheduleFile>(&contents)
                .with_context(|| format!("parsing TOML store at {}", path.display()))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(ScheduleFile::default()),
        Err(e) => Err(e).with_context(|| format!("reading store at {}", path.display())),
    }
}

/// Atomic write: write `data` to a sibling temp file, then rename into place.
fn atomic_write(target: &Path, data: &[u8]) -> Result<()> {
    let dir = target
        .parent()
        .ok_or_else(|| anyhow::anyhow!("target path has no parent dir"))?;

    // Create a named temp file in the same directory so rename is atomic.
    let mut tmp = tempfile_in(dir)?;
    tmp.1
        .write_all(data)
        .context("writing data to temp file")?;
    tmp.1.flush().context("flushing temp file")?;
    drop(tmp.1); // close before rename on some platforms

    fs::rename(&tmp.0, target)
        .with_context(|| format!("renaming {} -> {}", tmp.0.display(), target.display()))?;
    Ok(())
}

/// Returns `(temp_path, file)` for a new temp file inside `dir`.
fn tempfile_in(dir: &Path) -> Result<(PathBuf, fs::File)> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    // Simple collision-resistant name without an external crate.
    let name = format!(".wm-almanac-tmp-{ts}");
    let path = dir.join(name);
    let file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(|| format!("creating temp file {}", path.display()))?;
    Ok((path, file))
}

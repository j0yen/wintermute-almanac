//! `wm-almanac` — local recurring-routine store for the wintermute elder-care system.
//!
//! Subcommands: add, list, remove, enable, disable, next.

#![warn(missing_docs)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use std::process::ExitCode;

mod entry;
mod next;
mod store;

use entry::{Category, Entry, Recurrence, parse_hhmm, parse_recurrence};
use next::next_due;
use store::Store;

/// Local recurring-routine store for the wintermute elder-care system.
#[derive(Parser)]
#[command(name = "wm-almanac", version, author, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new routine entry.
    Add {
        /// Short human-readable label.
        #[arg(long)]
        label: String,

        /// Local fire time in HH:MM (24-hour).
        #[arg(long = "at")]
        at: String,

        /// Recurrence: `daily`, `mon,wed,fri` (weekdays), or `once:YYYY-MM-DD`.
        #[arg(long)]
        every: String,

        /// Text to speak/display when the routine fires.
        #[arg(long)]
        say: String,

        /// Category (med, meal, appt, activity).
        #[arg(long, value_enum, default_value = "activity")]
        category: Category,

        /// IANA timezone (defaults to $WM_TIMEZONE, then $TZ, then UTC).
        #[arg(long)]
        tz: Option<String>,

        /// Minutes to snooze.
        #[arg(long, default_value_t = 10)]
        snooze_min: u32,

        /// Maximum number of snoozes.
        #[arg(long, default_value_t = 2)]
        max_snoozes: u32,
    },

    /// List routine entries.
    List {
        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },

    /// Remove an entry by ID.
    Remove {
        /// Entry ID.
        id: String,
    },

    /// Enable an entry (sets opt_in=true).
    Enable {
        /// Entry ID.
        id: String,
    },

    /// Disable an entry (sets opt_in=false, entry is retained).
    Disable {
        /// Entry ID.
        id: String,
    },

    /// Print the next due entry and its fire timestamp.
    Next {
        /// Output format.
        #[arg(long, value_enum, default_value = "text")]
        format: OutputFormat,
    },
}

/// Output format for list/next subcommands.
#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    /// Human-readable text.
    Text,
    /// JSON array / object.
    Json,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Add {
            label,
            at,
            every,
            say,
            category,
            tz,
            snooze_min,
            max_snoozes,
        } => cmd_add(label, at, every, say, category, tz, snooze_min, max_snoozes),
        Commands::List { format } => cmd_list(format),
        Commands::Remove { id } => cmd_remove(id),
        Commands::Enable { id } => cmd_enable(id),
        Commands::Disable { id } => cmd_disable(id),
        Commands::Next { format } => cmd_next(format),
    }
}

fn resolve_tz(explicit: Option<String>) -> Result<String> {
    if let Some(z) = explicit {
        validate_tz(&z)?;
        return Ok(z);
    }
    // Try $WM_TIMEZONE, then $TZ, then fall back to UTC.
    for var in &["WM_TIMEZONE", "TZ"] {
        if let Ok(z) = std::env::var(var) {
            if !z.is_empty() {
                validate_tz(&z)?;
                return Ok(z);
            }
        }
    }
    Ok("UTC".to_owned())
}

fn validate_tz(tz: &str) -> Result<()> {
    tz.parse::<chrono_tz::Tz>()
        .map(|_| ())
        .map_err(|_| anyhow::anyhow!("invalid IANA timezone '{tz}'"))
}

#[allow(clippy::too_many_arguments)]
fn cmd_add(
    label: String,
    at: String,
    every: String,
    say: String,
    category: Category,
    tz: Option<String>,
    snooze_min: u32,
    max_snoozes: u32,
) -> Result<()> {
    // Validate inputs before touching the store.
    parse_hhmm(&at).map_err(|e| anyhow::anyhow!("{e}"))?;
    let recurrence: Recurrence =
        parse_recurrence(&every).map_err(|e| anyhow::anyhow!("{e}"))?;
    let resolved_tz = resolve_tz(tz)?;

    let entry = Entry {
        id: Entry::new_id(),
        label: label.clone(),
        say,
        recurrence,
        local_time: at,
        tz: resolved_tz,
        category,
        opt_in: true,
        snooze_min,
        max_snoozes,
        created_ms: Entry::now_ms(),
    };
    let id = entry.id.clone();

    let mut store = Store::open()?;
    store.add(entry)?;
    println!("added entry '{label}' with id {id}");
    Ok(())
}

fn cmd_list(format: OutputFormat) -> Result<()> {
    let store = Store::open()?;
    let entries = store.entries();
    match format {
        OutputFormat::Json => {
            let json =
                serde_json::to_string_pretty(entries).context("serialising entries to JSON")?;
            println!("{json}");
        }
        OutputFormat::Text => {
            if entries.is_empty() {
                println!("no entries");
            } else {
                for e in entries {
                    let status = if e.opt_in { "enabled" } else { "disabled" };
                    let rec = format_recurrence(&e.recurrence);
                    println!(
                        "[{}] {} | {} @ {} ({}) | {} | {}",
                        e.id, e.label, rec, e.local_time, e.tz, e.category, status
                    );
                }
            }
        }
    }
    Ok(())
}

fn format_recurrence(r: &Recurrence) -> String {
    match r {
        Recurrence::Daily => "daily".to_owned(),
        Recurrence::Weekly { days } => {
            let abbrevs: Vec<&str> = days.iter().map(weekday_abbrev).collect();
            abbrevs.join(",")
        }
        Recurrence::Once { date } => format!("once:{date}"),
    }
}

fn weekday_abbrev(w: &chrono::Weekday) -> &'static str {
    match w {
        chrono::Weekday::Mon => "mon",
        chrono::Weekday::Tue => "tue",
        chrono::Weekday::Wed => "wed",
        chrono::Weekday::Thu => "thu",
        chrono::Weekday::Fri => "fri",
        chrono::Weekday::Sat => "sat",
        chrono::Weekday::Sun => "sun",
    }
}

fn cmd_remove(id: String) -> Result<()> {
    let mut store = Store::open()?;
    let removed = store.remove(&id)?;
    if removed {
        println!("removed entry {id}");
    } else {
        eprintln!("no entry with id '{id}' found");
        return Err(anyhow::anyhow!("entry not found"));
    }
    Ok(())
}

fn cmd_enable(id: String) -> Result<()> {
    let mut store = Store::open()?;
    let found = store.set_opt_in(&id, true)?;
    if found {
        println!("enabled entry {id}");
    } else {
        eprintln!("no entry with id '{id}' found");
        return Err(anyhow::anyhow!("entry not found"));
    }
    Ok(())
}

fn cmd_disable(id: String) -> Result<()> {
    let mut store = Store::open()?;
    let found = store.set_opt_in(&id, false)?;
    if found {
        println!("disabled entry {id}");
    } else {
        eprintln!("no entry with id '{id}' found");
        return Err(anyhow::anyhow!("entry not found"));
    }
    Ok(())
}

fn cmd_next(format: OutputFormat) -> Result<()> {
    let store = Store::open()?;
    let due = next_due(store.entries())?;
    match format {
        OutputFormat::Json => {
            match &due {
                Some(nd) => {
                    let json =
                        serde_json::to_string_pretty(nd).context("serialising next-due to JSON")?;
                    println!("{json}");
                }
                None => {
                    println!("null");
                }
            }
        }
        OutputFormat::Text => match &due {
            Some(nd) => {
                println!("next: {} (id {}) fires at unix {}", nd.label, nd.id, nd.fire_ts_unix);
            }
            None => {
                println!("no entries due");
            }
        },
    }
    Ok(())
}

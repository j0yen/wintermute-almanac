//! Data model for a single recurring-routine entry.

use chrono::Weekday;
use serde::{Deserialize, Serialize};

/// The kind of routine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Recurrence {
    /// Fire every day at the scheduled time.
    Daily,
    /// Fire on specific weekdays.
    Weekly {
        /// Days of the week on which the routine fires.
        days: Vec<Weekday>,
    },
    /// Fire once on a specific date.
    Once {
        /// ISO 8601 date: `YYYY-MM-DD`.
        date: String,
    },
}

/// Category of the routine, used for filtering and display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Medication reminder.
    #[value(name = "med")]
    Med,
    /// Meal reminder.
    #[value(name = "meal")]
    Meal,
    /// Appointment reminder.
    #[value(name = "appt")]
    Appointment,
    /// Activity reminder.
    #[value(name = "activity")]
    Activity,
}

impl std::fmt::Display for Category {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Med => write!(f, "med"),
            Self::Meal => write!(f, "meal"),
            Self::Appointment => write!(f, "appt"),
            Self::Activity => write!(f, "activity"),
        }
    }
}

/// A single scheduled routine entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Unique identifier (ULID-style: timestamp-random hex string).
    pub id: String,
    /// Human-readable label for the routine.
    pub label: String,
    /// Text to speak or display when the routine fires.
    pub say: String,
    /// Recurrence pattern.
    pub recurrence: Recurrence,
    /// Local time in `HH:MM` 24-hour format.
    pub local_time: String,
    /// IANA timezone identifier.
    pub tz: String,
    /// Category of routine.
    pub category: Category,
    /// Whether the routine is enabled (opt-in).
    pub opt_in: bool,
    /// Minutes to snooze when acknowledged.
    pub snooze_min: u32,
    /// Maximum number of snoozes allowed.
    pub max_snoozes: u32,
    /// Creation timestamp in milliseconds since Unix epoch.
    pub created_ms: u64,
}

impl Entry {
    /// Generate a new unique ID using timestamp + pseudo-random suffix.
    #[must_use]
    pub fn new_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        // Use a simple deterministic suffix derived from the timestamp to avoid
        // pulling in a UUID crate. Collisions are astronomically unlikely for a
        // local store that rarely has more than tens of entries.
        let hi = (ts >> 16) as u32;
        let lo = (ts & 0xFFFF) as u32;
        let suffix = hi.wrapping_mul(0x9e37_79b9).wrapping_add(lo);
        format!("{ts:013x}{suffix:08x}")
    }

    /// Return current epoch-millis timestamp.
    #[must_use]
    pub fn now_ms() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// Parse an `HH:MM` string into `(hour, minute)`.
///
/// # Errors
/// Returns an error if the string is not valid `HH:MM` or if the values are
/// out of range (hour ≥ 24, minute ≥ 60).
pub fn parse_hhmm(s: &str) -> Result<(u8, u8), String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid time '{s}': expected HH:MM format"));
    }
    let hour: u8 = parts[0]
        .parse()
        .map_err(|_| format!("invalid hour in '{s}'"))?;
    let minute: u8 = parts[1]
        .parse()
        .map_err(|_| format!("invalid minute in '{s}'"))?;
    if hour >= 24 {
        return Err(format!("invalid hour {hour} in '{s}': must be 0-23"));
    }
    if minute >= 60 {
        return Err(format!("invalid minute {minute} in '{s}': must be 0-59"));
    }
    Ok((hour, minute))
}

/// Parse a recurrence string into a [`Recurrence`] value.
///
/// Accepted formats:
/// - `daily`
/// - Comma-separated weekday abbreviations: `mon,wed,fri`
/// - `once:YYYY-MM-DD`
///
/// # Errors
/// Returns an error if the string is not one of the accepted formats.
pub fn parse_recurrence(s: &str) -> Result<Recurrence, String> {
    if s.eq_ignore_ascii_case("daily") {
        return Ok(Recurrence::Daily);
    }
    if let Some(date) = s.strip_prefix("once:") {
        validate_date(date)?;
        return Ok(Recurrence::Once {
            date: date.to_owned(),
        });
    }
    // Try weekday list.
    let days: Result<Vec<Weekday>, String> = s
        .split(',')
        .map(|part| parse_weekday(part.trim()))
        .collect();
    match days {
        Ok(d) if !d.is_empty() => Ok(Recurrence::Weekly { days: d }),
        Ok(_) => Err(format!("empty weekday list in '{s}'")),
        Err(e) => Err(e),
    }
}

fn parse_weekday(s: &str) -> Result<Weekday, String> {
    match s.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        _ => Err(format!("unknown weekday '{s}': use mon/tue/wed/thu/fri/sat/sun")),
    }
}

fn validate_date(date: &str) -> Result<(), String> {
    // Simple structural validation: YYYY-MM-DD
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return Err(format!(
            "invalid date '{date}': expected YYYY-MM-DD format"
        ));
    }
    let year: u32 = parts[0]
        .parse()
        .map_err(|_| format!("invalid year in date '{date}'"))?;
    let month: u8 = parts[1]
        .parse()
        .map_err(|_| format!("invalid month in date '{date}'"))?;
    let day: u8 = parts[2]
        .parse()
        .map_err(|_| format!("invalid day in date '{date}'"))?;
    if year < 2000 || year > 9999 {
        return Err(format!("year {year} out of range in date '{date}'"));
    }
    if !(1..=12).contains(&month) {
        return Err(format!("month {month} out of range in date '{date}'"));
    }
    if !(1..=31).contains(&day) {
        return Err(format!("day {day} out of range in date '{date}'"));
    }
    Ok(())
}

//! Computation of the next-fire instant for the soonest enabled entry.

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use serde::Serialize;

use crate::entry::{Entry, Recurrence};

/// The result returned by [`next_due`].
#[derive(Debug, Serialize)]
pub struct NextDue {
    /// Entry ID.
    pub id: String,
    /// Next fire as Unix timestamp (seconds).
    pub fire_ts_unix: i64,
    /// Human-readable label.
    pub label: String,
}

/// Compute the next-fire instant across all enabled entries.
///
/// Returns `None` when the store is empty or every entry is disabled.
///
/// # Errors
/// Returns an error if an entry's timezone string is not a valid IANA zone.
pub fn next_due(entries: &[Entry]) -> Result<Option<NextDue>> {
    let now = Utc::now();
    let mut best: Option<(DateTime<Utc>, &Entry)> = None;

    for entry in entries {
        if !entry.opt_in {
            continue;
        }
        let fire = next_fire(entry, now).with_context(|| {
            format!(
                "computing next fire for entry '{}' (id {})",
                entry.label, entry.id
            )
        })?;
        if let Some(fire) = fire {
            if best.is_none() || fire < best.as_ref().map(|(t, _)| *t).unwrap_or(fire) {
                best = Some((fire, entry));
            }
        }
    }

    Ok(best.map(|(fire, entry)| NextDue {
        id: entry.id.clone(),
        fire_ts_unix: fire.timestamp(),
        label: entry.label.clone(),
    }))
}

/// Compute the next fire instant for a single entry relative to `now`.
///
/// Returns `None` if the entry is a `Once` event already in the past.
///
/// # Errors
/// Returns an error if the timezone string is invalid.
fn next_fire(entry: &Entry, now: DateTime<Utc>) -> Result<Option<DateTime<Utc>>> {
    let tz: Tz = entry
        .tz
        .parse::<Tz>()
        .map_err(|_| anyhow::anyhow!("invalid timezone '{}'", entry.tz))?;

    // Parse HH:MM
    let colon = entry
        .local_time
        .find(':')
        .ok_or_else(|| anyhow::anyhow!("malformed local_time '{}'", entry.local_time))?;
    let hour: u32 = entry.local_time[..colon]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid hour in local_time '{}'", entry.local_time))?;
    let minute: u32 = entry.local_time[colon + 1..]
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid minute in local_time '{}'", entry.local_time))?;

    match &entry.recurrence {
        Recurrence::Daily => {
            let candidate = next_daily(now, tz, hour, minute)?;
            Ok(Some(candidate))
        }
        Recurrence::Weekly { days } => {
            let candidate = next_weekly(now, tz, hour, minute, days)?;
            Ok(Some(candidate))
        }
        Recurrence::Once { date } => {
            let nd = NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .with_context(|| format!("parsing date '{date}'"))?;
            let naive_dt = nd
                .and_hms_opt(hour, minute, 0)
                .ok_or_else(|| anyhow::anyhow!("invalid time {hour}:{minute}"))?;
            // Convert to UTC via the timezone, handling ambiguous/skipped times by
            // taking the earliest valid instant.
            let local_dt = match tz.from_local_datetime(&naive_dt) {
                chrono::LocalResult::Single(dt) => dt,
                chrono::LocalResult::Ambiguous(earliest, _) => earliest,
                chrono::LocalResult::None => {
                    // Time was skipped due to DST gap — advance by 1 hour.
                    let adjusted = naive_dt + chrono::Duration::hours(1);
                    tz.from_local_datetime(&adjusted)
                        .single()
                        .ok_or_else(|| anyhow::anyhow!("cannot resolve local time in tz '{}'", entry.tz))?
                }
            };
            let utc_dt = local_dt.with_timezone(&Utc);
            if utc_dt <= now {
                Ok(None) // already passed
            } else {
                Ok(Some(utc_dt))
            }
        }
    }
}

fn next_daily(
    now: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
) -> Result<DateTime<Utc>> {
    let local_now = now.with_timezone(&tz);
    let today = local_now.date_naive();
    let candidate_naive = today
        .and_hms_opt(hour, minute, 0)
        .ok_or_else(|| anyhow::anyhow!("invalid time {hour}:{minute}"))?;
    let candidate_utc = naive_to_utc(candidate_naive, tz)?;
    if candidate_utc > now {
        Ok(candidate_utc)
    } else {
        // Tomorrow
        let tomorrow = today.succ_opt().ok_or_else(|| anyhow::anyhow!("date overflow"))?;
        let next_naive = tomorrow
            .and_hms_opt(hour, minute, 0)
            .ok_or_else(|| anyhow::anyhow!("invalid time {hour}:{minute}"))?;
        naive_to_utc(next_naive, tz)
    }
}

fn next_weekly(
    now: DateTime<Utc>,
    tz: Tz,
    hour: u32,
    minute: u32,
    days: &[Weekday],
) -> Result<DateTime<Utc>> {
    if days.is_empty() {
        return Err(anyhow::anyhow!("weekly recurrence has no days"));
    }
    let local_now = now.with_timezone(&tz);
    let today = local_now.date_naive();

    // Search up to 8 days ahead to find the next matching weekday
    for offset in 0u32..=7 {
        let candidate_date = today
            .checked_add_days(chrono::Days::new(u64::from(offset)))
            .ok_or_else(|| anyhow::anyhow!("date overflow at offset {offset}"))?;
        let candidate_weekday = candidate_date.weekday();
        if days.contains(&candidate_weekday) {
            let candidate_naive = candidate_date
                .and_hms_opt(hour, minute, 0)
                .ok_or_else(|| anyhow::anyhow!("invalid time {hour}:{minute}"))?;
            let candidate_utc = naive_to_utc(candidate_naive, tz)?;
            if candidate_utc > now {
                return Ok(candidate_utc);
            }
        }
    }
    Err(anyhow::anyhow!(
        "could not find a next weekly fire time in the next 8 days"
    ))
}

fn naive_to_utc(naive: chrono::NaiveDateTime, tz: Tz) -> Result<DateTime<Utc>> {
    let local = match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(earliest, _) => earliest,
        chrono::LocalResult::None => {
            let adjusted = naive + chrono::Duration::hours(1);
            tz.from_local_datetime(&adjusted)
                .single()
                .ok_or_else(|| anyhow::anyhow!("cannot resolve local time"))?
        }
    };
    Ok(local.with_timezone(&Utc))
}

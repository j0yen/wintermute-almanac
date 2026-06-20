# wintermute-almanac

The schedule a voice-AI elder-care system reads from: a local, offline store of recurring routines — "the blue pill, every morning at 8."

Everything else in the fleet can speak and listen, but nothing remembered *when* to speak. `wm-almanac` is that memory: a durable, credential-free CLI for recurring routines (medications, meals, appointments, activities), held in one TOML file with no network, no bus, and no SecretService. A small `daemon` mode sleeps until the next entry is due and publishes it. The rest of the almanac work builds on this schedule model.

## Features

- **Durable TOML store** at `$XDG_DATA_HOME/wm-almanac/schedule.toml` — atomic writes, tolerates missing file
- **Recurrence types**: `daily`, `weekly` (specific weekdays), `once` (one-shot date)
- **Categories**: `med`, `meal`, `appt`, `activity`
- **Per-entry controls**: enable/disable (retains entry), snooze minutes, max snoozes
- **Next-due computation**: DST-correct via `chrono-tz`, returns soonest enabled entry

## Install

```sh
cargo install --path .
# or
cargo build --release && install -Dm755 target/release/wm-almanac ~/.local/bin/wm-almanac
```

## Usage

```sh
# Add a daily medication reminder
wm-almanac add --label "morning pills" --at 08:00 --every daily \
    --say "time for your blue pill" --category med

# Add a weekly activity (Mon/Wed/Fri)
wm-almanac add --label "morning walk" --at 09:00 --every mon,wed,fri \
    --say "time for a walk" --category activity

# Add a one-time appointment
wm-almanac add --label "doctor visit" --at 14:00 --every once:2026-06-15 \
    --say "doctor appointment today" --category appt

# List all entries (text or JSON)
wm-almanac list
wm-almanac list --format json

# Disable / re-enable an entry
wm-almanac disable <id>
wm-almanac enable <id>

# Remove an entry
wm-almanac remove <id>

# Next due entry (used by the almanac tick daemon)
wm-almanac next
wm-almanac next --format json   # → {"id":"…","fire_ts_unix":…,"label":"…"}

# Tick daemon: fire at most one due entry and exit (for a systemd-user timer)
wm-almanac daemon --once
# Or run it long-lived; it loops until SIGINT/SIGTERM
wm-almanac daemon
```

When an entry fires, the daemon publishes `wm.almanac.due`; an unacknowledged
entry surfaces as `wm.almanac.missed`, and a missed `med`-category entry also
emits `wm.family.message` to the caregiver. A publish failure degrades to
`wm.health.almanac` rather than dropping silently. The systemd units in
[`contrib/systemd/`](contrib/systemd) run the `--once` tick on a timer.

## Where it fits

Part of the wintermute voice-AI elder-care fleet. `wm-almanac` holds the
schedule; the tick daemon turns due entries into bus messages other fleet
components speak and act on. The kin bridge (`wm.family.message`) is how a
missed medication reaches a family caregiver.

## Recent

- **v0.4.0** (2026-05-30): Missed almanac entries surface to the caregiver via `wm.almanac.missed`; kin bridge emits `wm.family.message` for med-category misses; publish failures degrade to `wm.health.almanac` (no silent drops).
- **v0.3.0** (2026-05-29): Completed almanac-tick-daemon AC4 — injected-clock re-arm test (`ac4_daily_entry_rearms_to_next_day`) proves `Daily` entries re-arm to next day via `soonest_enabled`; adds `FixedClock` test helper and `Clock`/`SystemClock` extension points.
- **v0.2.0** (2026-05-29): Added `wm-almanac daemon` tick mode — `--once` for systemd-user timer, long-running loop with SIGINT/SIGTERM, `PublishSink` trait, DST-correct re-arm, `wm.health.almanac` degrade hooks.

## Acceptance tests

All 9 acceptance criteria from the PRD pass under `cargo test --release`:

| AC | Description |
|----|-------------|
| 1  | `add` exits 0 and persists entry to `schedule.toml` |
| 2  | `list --format json` returns JSON array; `--format text` prints human lines |
| 3  | `remove <id>` deletes only that entry |
| 4  | `disable` / `enable` toggle `opt_in`; entry retained |
| 5  | `--every mon,wed,fri` → Weekly; `--every once:DATE` → Once; invalid → non-zero |
| 6  | Invalid `--at` or `--tz` exits non-zero without writing store |
| 7  | `next --format json` returns `{id, fire_ts_unix, label}`; empty/disabled → null |
| 8  | Store writes are atomic (temp + rename); missing file → empty schedule |
| 9  | `wm-almanac --help` lists all subcommands |

## License

MIT OR Apache-2.0

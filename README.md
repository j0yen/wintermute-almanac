# wintermute-almanac

Local, offline store of recurring routine entries for the wintermute elder-care system.

Wintermute has no place to record "the blue pill, every morning at 8." This crate creates that place. `wm-almanac` is a durable, credential-free CLI for managing scheduled routines (medications, meals, appointments, activities) — no network, no bus, no SecretService. It is the schedule model every other almanac PRD builds on.

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
```

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

# Changelog

## v0.4.0 — 2026-05-30

Missed almanac entries surface to the caregiver. A missed acknowledgment (especially category=med) emits a normalized wm.almanac.missed envelope unconditionally, and bridges to wm.family.message via the kin channel when configured. Non-med categories require notify_on_miss=true to bridge. Bridge publish failures emit wm.health.almanac so silent medication-miss drops are impossible.

## v0.3.0 — 2026-05-29

Complete PRD-almanac-tick-daemon acceptance criteria with injected-clock re-arm test (AC4), FixedClock test helper, and allow(dead_code) on Clock/SystemClock extension points. Adds ac4_daily_entry_rearms_to_next_day test proving re-arm works via soonest_enabled with two advancing now values.

## v0.2.0 — 2026-05-29

Add `wm-almanac daemon` tick-daemon mode that fires entries at their scheduled
local time and publishes `wm.almanac.due` to agorabus (PRD-almanac-tick-daemon):

- `src/daemon.rs`: `run_once` (for systemd-user timer) and `run_daemon`
  (long-running loop with SIGINT/SIGTERM handling). `PublishSink` trait
  lets tests capture envelopes without a live bus.
- `src/next.rs`: `next_fire_for()` extracted as a shared function.
- `contrib/systemd/wm-almanac-tick.{timer,service}`: drive `--once` every
  minute via systemd-user timer.
- Degrade-out-loud: unreadable store or clock-skew (>1 day past) emits
  `wm.health.almanac` rather than silently skipping.

## v0.1.0 — initial release

Local offline TOML store for recurring routine entries (`add`, `list`,
`remove`, `enable`, `disable`, `next`). DST-correct recurrence (Daily,
Weekly, Once) via chrono-tz.

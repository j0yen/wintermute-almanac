# Changelog

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

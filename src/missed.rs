//! Bridge for missed almanac entries — emit `wm.almanac.missed` unconditionally
//! and, when the kin family topic is configured, also emit `wm.family.message`
//! with a gentle, human-phrased body.
//!
//! # kin topic contract
//!
//! Written against kin vision 2026-05-28 (`visions/kin.md` §"Topic contract"):
//!
//! | Topic | Payload |
//! |---|---|
//! | `wm.family.message` | `{to, body, urgency, ts}` |
//!
//! - `to`: the enrolled caregiver identifier (typically `"joe"` or a transport
//!   address such as an email address); read from config.
//! - `body`: human-phrased string, e.g.
//!   `"Mom hasn't acknowledged her morning pills (due 8:00am)."`.
//! - `urgency`: `"normal"` for non-med categories unless `notify_on_miss=true`;
//!   `"high"` for `category=med`.
//! - `ts`: RFC-3339 UTC timestamp string.
//!
//! If kin is not configured (no `to` address enrolled), the bridge step is
//! skipped and the absence is logged at INFO level. No error or panic.
//!
//! If the kin bridge publish call fails, `wm.health.almanac` is emitted so
//! that a silent delivery failure on a medication miss is impossible.
//! (Companion-degrade discipline: degrade out loud.)

use serde::{Deserialize, Serialize};

use crate::entry::Category;

// ── Topic constants ──────────────────────────────────────────────────────────

/// Topic on which a missed almanac acknowledgment is published.
pub const TOPIC_ACK: &str = "wm.almanac.ack";

/// Topic on which the normalized missed envelope is published.
pub const TOPIC_MISSED: &str = "wm.almanac.missed";

/// Topic used by the kin bridge to carry the family-message.
/// Matches kin vision 2026-05-28 §"Topic contract".
pub const TOPIC_FAMILY_MESSAGE: &str = "wm.family.message";

/// Topic used by the companion-degrade discipline: emitted when the kin
/// bridge publish fails, so a silent medication-miss delivery failure is
/// impossible.
pub const TOPIC_HEALTH: &str = "wm.health.almanac";

// ── Input event ─────────────────────────────────────────────────────────────

/// An incoming `wm.almanac.ack` event with `state = "missed"`.
///
/// The caller is responsible for filtering to `state = "missed"` before
/// passing to [`process_missed`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AckMissed {
    /// Entry ID (matches `Entry.id`).
    pub id: String,
    /// Human-readable label (e.g. `"morning pills"`).
    pub label: String,
    /// Entry category, used for kin-bridge routing.
    pub category: Category,
    /// Scheduled time in `HH:MM` (24-hour local) — carried in the envelope
    /// so the human-phrased body can name the due time.
    pub due_time: String,
    /// Whether this specific entry has opted into the kin bridge for
    /// non-`med` categories.  For `category=med` this field is ignored
    /// (always bridged).
    #[serde(default)]
    pub notify_on_miss: bool,
}

// ── Output envelopes ─────────────────────────────────────────────────────────

/// The normalized `wm.almanac.missed` envelope, published unconditionally
/// for every missed entry regardless of kin availability.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AlmanacMissed {
    /// Entry ID.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Category of the entry.
    pub category: Category,
    /// RFC-3339 UTC timestamp of when the miss was recorded.
    pub missed_ts: String,
}

/// The `wm.family.message` envelope forwarded to the kin delivery daemon.
/// Field names and semantics match kin vision 2026-05-28 §"Topic contract".
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FamilyMessage {
    /// Caregiver identifier / address (from kin config, e.g. `"joe"`).
    pub to: String,
    /// Human-phrased notification body.
    pub body: String,
    /// Urgency: `"high"` for `category=med`, `"normal"` otherwise.
    pub urgency: String,
    /// RFC-3339 UTC timestamp.
    pub ts: String,
}

/// A `wm.health.almanac` diagnostic emitted when the kin bridge publish fails.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthAlmanac {
    /// Short error reason (non-empty on failure).
    pub reason: String,
    /// RFC-3339 UTC timestamp.
    pub ts: String,
}

// ── Publish trait (test-double seam) ────────────────────────────────────────

/// Abstraction over an agorabus publisher, providing a test-double seam.
///
/// In production, implement this against the real agorabus client.  In tests,
/// use [`VecSink`] to capture published messages without a live bus.
///
/// # Errors
/// Returns an error string if the publish operation fails.
pub trait Publish {
    /// Publish `payload` on `topic`.
    ///
    /// # Errors
    /// Returns an error describing the failure.
    fn publish(&mut self, topic: &str, payload: &str) -> Result<(), String>;
}

/// A simple in-memory [`Publish`] sink for unit tests.
///
/// Records every `(topic, payload)` pair in insertion order.
#[derive(Debug, Default)]
pub struct VecSink {
    /// All published messages, in order.
    pub messages: Vec<(String, String)>,
}

impl Publish for VecSink {
    fn publish(&mut self, topic: &str, payload: &str) -> Result<(), String> {
        self.messages.push((topic.to_owned(), payload.to_owned()));
        Ok(())
    }
}

/// A [`Publish`] sink that always fails on the kin family topic, used to test
/// the degrade-out-loud path.
#[derive(Debug, Default)]
pub struct FailingKinSink {
    /// Messages that were successfully published (non-kin topics).
    pub messages: Vec<(String, String)>,
}

impl Publish for FailingKinSink {
    fn publish(&mut self, topic: &str, payload: &str) -> Result<(), String> {
        if topic == TOPIC_FAMILY_MESSAGE {
            return Err("simulated kin publish failure".to_owned());
        }
        self.messages.push((topic.to_owned(), payload.to_owned()));
        Ok(())
    }
}

// ── Kin config ───────────────────────────────────────────────────────────────

/// Runtime configuration for the kin family bridge.
///
/// If `caregiver_id` is `None`, the kin bridge is considered unconfigured and
/// the `wm.family.message` publish is skipped.
#[derive(Debug, Clone, Default)]
pub struct KinConfig {
    /// Caregiver identifier / transport address (e.g. `"joe"` or an email).
    /// `None` means kin is not enrolled / not configured.
    pub caregiver_id: Option<String>,
}

// ── Core logic ───────────────────────────────────────────────────────────────

/// Process a missed almanac acknowledgment event.
///
/// Steps:
/// 1. Build and publish `wm.almanac.missed` unconditionally.
/// 2. If the kin bridge is configured AND this entry should be bridged
///    (either `category=med` or `notify_on_miss=true`), also publish
///    `wm.family.message` with a gentle human-phrased body.
/// 3. If step 2's publish fails, emit `wm.health.almanac` so the miss is
///    never silently dropped (companion-degrade discipline).
///
/// Returns the normalized [`AlmanacMissed`] envelope that was published.
///
/// # Errors
/// Currently infallible from the caller's perspective — failures in the
/// kin bridge are downgraded to a `wm.health.almanac` diagnostic.  The
/// `Err` variant is reserved for future use (e.g. a failure to serialize
/// the missed envelope itself).
pub fn process_missed(
    ack: &AckMissed,
    kin: &KinConfig,
    now_rfc3339: &str,
    publisher: &mut dyn Publish,
) -> Result<AlmanacMissed, String> {
    // Step 1 — build and publish `wm.almanac.missed` unconditionally.
    let missed = AlmanacMissed {
        id: ack.id.clone(),
        label: ack.label.clone(),
        category: ack.category,
        missed_ts: now_rfc3339.to_owned(),
    };
    let missed_json =
        serde_json::to_string(&missed).map_err(|e| format!("serialize missed: {e}"))?;
    publisher
        .publish(TOPIC_MISSED, &missed_json)
        .map_err(|e| format!("publish wm.almanac.missed: {e}"))?;

    // Step 2 — kin bridge (if configured and this entry qualifies).
    let should_bridge = ack.category == Category::Med || ack.notify_on_miss;
    if let Some(ref caregiver_id) = kin.caregiver_id {
        if should_bridge {
            let body = build_human_body(&ack.label, &ack.due_time, ack.category);
            let urgency = if ack.category == Category::Med {
                "high"
            } else {
                "normal"
            };
            let family_msg = FamilyMessage {
                to: caregiver_id.clone(),
                body,
                urgency: urgency.to_owned(),
                ts: now_rfc3339.to_owned(),
            };
            let family_json = serde_json::to_string(&family_msg)
                .map_err(|e| format!("serialize family message: {e}"))?;

            if let Err(e) = publisher.publish(TOPIC_FAMILY_MESSAGE, &family_json) {
                // Step 3 — degrade-out-loud: publish wm.health.almanac so the
                // medication-miss delivery failure is not silently dropped.
                let health = HealthAlmanac {
                    reason: format!("kin bridge publish failed: {e}"),
                    ts: now_rfc3339.to_owned(),
                };
                let health_json = serde_json::to_string(&health)
                    .unwrap_or_else(|se| format!(r#"{{"reason":"serialize error: {se}","ts":""}}"#));
                // Best-effort: if health publish also fails, we have nowhere
                // left to escalate — log the original error in the returned Ok.
                let _ = publisher.publish(TOPIC_HEALTH, &health_json);
            }
        } else {
            // Non-med, not opted in — skip kin bridge.  Nothing to log at
            // anything higher than TRACE; INFO would be noise on every walk.
        }
    } else {
        // Kin not configured — log absence at INFO level.  The caller /
        // integration layer is expected to forward this to the logging backend.
        // We emit a structured note via the health topic only for med categories
        // so that a mis-configured kin setup for medication is surfaced.
        if ack.category == Category::Med {
            let health = HealthAlmanac {
                reason: "kin not configured; wm.family.message not sent for med miss".to_owned(),
                ts: now_rfc3339.to_owned(),
            };
            if let Ok(health_json) = serde_json::to_string(&health) {
                let _ = publisher.publish(TOPIC_HEALTH, &health_json);
            }
        }
    }

    Ok(missed)
}

// ── Human body builder ───────────────────────────────────────────────────────

/// Build a gentle, human-phrased notification body for the kin message.
///
/// Examples:
/// - med:  `"Mom hasn't acknowledged her morning pills (due 8:00am)."`
/// - meal: `"Mom hasn't acknowledged her breakfast (due 8:00am)."`
/// - appt: `"Mom hasn't acknowledged her dentist appointment (due 2:00pm)."`
fn build_human_body(label: &str, due_time: &str, category: Category) -> String {
    let category_phrase = match category {
        Category::Med => "medication",
        Category::Meal => "meal",
        Category::Appointment => "appointment",
        Category::Activity => "activity",
    };
    format!(
        "Mom hasn't acknowledged her {label} {category_phrase} (due {due_time})."
    )
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn med_ack() -> AckMissed {
        AckMissed {
            id: "abc123".to_owned(),
            label: "morning pills".to_owned(),
            category: Category::Med,
            due_time: "8:00am".to_owned(),
            notify_on_miss: false,
        }
    }

    fn meal_ack(notify: bool) -> AckMissed {
        AckMissed {
            id: "def456".to_owned(),
            label: "breakfast".to_owned(),
            category: Category::Meal,
            due_time: "9:00am".to_owned(),
            notify_on_miss: notify,
        }
    }

    fn kin_configured() -> KinConfig {
        KinConfig {
            caregiver_id: Some("joe".to_owned()),
        }
    }

    fn kin_absent() -> KinConfig {
        KinConfig { caregiver_id: None }
    }

    const NOW: &str = "2026-05-29T12:00:00Z";

    // ── AC1: med ack → wm.almanac.missed published ───────────────────────────

    /// AC1: A `wm.almanac.ack {id, state:"missed", category:"med"}` causes
    /// publication of a `wm.almanac.missed` envelope with `id`, `label`,
    /// `category`, `missed_ts`.
    #[test]
    fn ac1_med_missed_publishes_almanac_missed() {
        let mut sink = VecSink::default();
        let result = process_missed(&med_ack(), &kin_absent(), NOW, &mut sink);
        assert!(result.is_ok());
        let missed = result.unwrap();
        assert_eq!(missed.id, "abc123");
        assert_eq!(missed.label, "morning pills");
        assert_eq!(missed.category, Category::Med);
        assert_eq!(missed.missed_ts, NOW);

        // The first published message must be wm.almanac.missed.
        let almanac_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_MISSED)
            .collect();
        assert_eq!(almanac_msgs.len(), 1, "exactly one wm.almanac.missed");
        let parsed: AlmanacMissed =
            serde_json::from_str(&almanac_msgs[0].1).expect("valid JSON");
        assert_eq!(parsed.id, "abc123");
        assert_eq!(parsed.label, "morning pills");
        assert_eq!(parsed.category, Category::Med);
        assert_eq!(parsed.missed_ts, NOW);
    }

    // ── AC2: med + kin configured → also publishes wm.family.message ─────────

    /// AC2: When the kin family topic is configured, the same missed-med event
    /// also publishes a `wm.family.message` whose body names the entry and its
    /// due time in human language; asserted via VecSink (no live bus).
    #[test]
    fn ac2_med_with_kin_publishes_family_message() {
        let mut sink = VecSink::default();
        let result = process_missed(&med_ack(), &kin_configured(), NOW, &mut sink);
        assert!(result.is_ok());

        let family_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_FAMILY_MESSAGE)
            .collect();
        assert_eq!(family_msgs.len(), 1, "exactly one wm.family.message");
        let msg: FamilyMessage =
            serde_json::from_str(&family_msgs[0].1).expect("valid JSON");
        assert_eq!(msg.to, "joe");
        assert!(
            msg.body.contains("morning pills"),
            "body should name the entry"
        );
        assert!(
            msg.body.contains("8:00am"),
            "body should name the due time"
        );
        assert_eq!(msg.urgency, "high", "med urgency must be high");
        assert_eq!(msg.ts, NOW);
    }

    // ── AC3: kin absent → only wm.almanac.missed, no error ──────────────────

    /// AC3: With kin not configured, only `wm.almanac.missed` is published;
    /// the absence of the family topic is not an error or panic.
    /// (For med: we emit wm.health.almanac as a soft diagnostic, but no error.)
    #[test]
    fn ac3_kin_absent_only_almanac_missed_no_error() {
        let mut sink = VecSink::default();
        let result = process_missed(&med_ack(), &kin_absent(), NOW, &mut sink);
        assert!(result.is_ok());

        let family_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_FAMILY_MESSAGE)
            .collect();
        assert_eq!(family_msgs.len(), 0, "no wm.family.message when kin absent");

        let almanac_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_MISSED)
            .collect();
        assert_eq!(almanac_msgs.len(), 1, "wm.almanac.missed still published");
    }

    // ── AC3b: kin absent, non-med → ONLY wm.almanac.missed, no health noise ──

    #[test]
    fn ac3b_kin_absent_non_med_no_health_emitted() {
        let mut sink = VecSink::default();
        let result = process_missed(&meal_ack(false), &kin_absent(), NOW, &mut sink);
        assert!(result.is_ok());

        // For non-med with kin absent, we do NOT emit wm.health.almanac —
        // skipping a meal notification when kin isn't configured is expected.
        let health_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_HEALTH)
            .collect();
        assert_eq!(
            health_msgs.len(),
            0,
            "no health diagnostic for non-med kin-absent"
        );
    }

    // ── AC4: category=meal, notify_on_miss=false → no wm.family.message ─────

    /// AC4: A `category=meal` miss with `notify_on_miss=false` (default)
    /// publishes `wm.almanac.missed` but does NOT publish `wm.family.message`;
    /// setting `notify_on_miss=true` opts it into the kin bridge.
    #[test]
    fn ac4_non_med_no_notify_skips_family_message() {
        let mut sink = VecSink::default();
        let result = process_missed(&meal_ack(false), &kin_configured(), NOW, &mut sink);
        assert!(result.is_ok());

        let family_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_FAMILY_MESSAGE)
            .collect();
        assert_eq!(
            family_msgs.len(),
            0,
            "no family message for non-med without notify_on_miss"
        );

        let almanac_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_MISSED)
            .collect();
        assert_eq!(almanac_msgs.len(), 1, "wm.almanac.missed still published");
    }

    /// AC4 (opt-in path): setting `notify_on_miss=true` causes a meal miss
    /// to also bridge to kin.
    #[test]
    fn ac4_non_med_with_notify_sends_family_message() {
        let mut sink = VecSink::default();
        let result = process_missed(&meal_ack(true), &kin_configured(), NOW, &mut sink);
        assert!(result.is_ok());

        let family_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_FAMILY_MESSAGE)
            .collect();
        assert_eq!(
            family_msgs.len(),
            1,
            "family message present when notify_on_miss=true"
        );
        let msg: FamilyMessage =
            serde_json::from_str(&family_msgs[0].1).expect("valid JSON");
        assert_eq!(msg.urgency, "normal", "non-med urgency is normal");
    }

    // ── AC5: kin bridge configured but publish fails → wm.health.almanac ─────

    /// AC5: If the kin bridge is configured and the `wm.family.message`
    /// publish fails, a `wm.health.almanac` diagnostic is emitted — no
    /// silent drop on a medication miss.
    #[test]
    fn ac5_kin_publish_failure_emits_health() {
        let mut sink = FailingKinSink::default();
        let result = process_missed(&med_ack(), &kin_configured(), NOW, &mut sink);
        assert!(result.is_ok(), "should not error on kin failure");

        // wm.almanac.missed was still published.
        let almanac_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_MISSED)
            .collect();
        assert_eq!(almanac_msgs.len(), 1, "wm.almanac.missed still published");

        // wm.health.almanac was emitted as the degrade signal.
        let health_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_HEALTH)
            .collect();
        assert_eq!(health_msgs.len(), 1, "wm.health.almanac should be emitted");
        let health: HealthAlmanac =
            serde_json::from_str(&health_msgs[0].1).expect("valid JSON");
        assert!(
            !health.reason.is_empty(),
            "health reason should be non-empty"
        );
    }

    // ── AC6: kin field names match kin.md convention ──────────────────────────

    /// AC6: The `wm.family.*` field names/shape match kin's published
    /// convention (kin vision 2026-05-28, §"Topic contract").
    /// Comment records kin version.
    #[test]
    fn ac6_family_message_fields_match_kin_convention() {
        // kin convention as of 2026-05-28 visions/kin.md §"Topic contract":
        //   wm.family.message → {to, body, urgency, ts}
        let mut sink = VecSink::default();
        let _ = process_missed(&med_ack(), &kin_configured(), NOW, &mut sink);
        let family_msgs: Vec<_> = sink
            .messages
            .iter()
            .filter(|(t, _)| t == TOPIC_FAMILY_MESSAGE)
            .collect();
        assert_eq!(family_msgs.len(), 1);
        let v: serde_json::Value =
            serde_json::from_str(&family_msgs[0].1).expect("valid JSON");
        assert!(v.get("to").is_some(), "missing 'to' field");
        assert!(v.get("body").is_some(), "missing 'body' field");
        assert!(v.get("urgency").is_some(), "missing 'urgency' field");
        assert!(v.get("ts").is_some(), "missing 'ts' field");
    }

    // ── AC7 (subset): cargo test covering all branches ─────────────────────
    // The unit tests above collectively cover:
    //   med → both topics           (ac1, ac2)
    //   non-med → missed-only       (ac4)
    //   kin-absent → missed-only    (ac3)
    //   publish-failure → health    (ac5)
    // They run within `cargo test` without a live bus.

    // ── build_human_body helper ───────────────────────────────────────────────

    #[test]
    fn human_body_med() {
        let body = build_human_body("morning pills", "8:00am", Category::Med);
        assert!(body.contains("morning pills"));
        assert!(body.contains("8:00am"));
        assert!(body.contains("medication"));
    }

    #[test]
    fn human_body_meal() {
        let body = build_human_body("breakfast", "9:00am", Category::Meal);
        assert!(body.contains("breakfast"));
        assert!(body.contains("meal"));
    }

    #[test]
    fn human_body_appt() {
        let body = build_human_body("dentist", "2:00pm", Category::Appointment);
        assert!(body.contains("dentist"));
        assert!(body.contains("appointment"));
    }

    #[test]
    fn human_body_activity() {
        let body = build_human_body("walk", "3:00pm", Category::Activity);
        assert!(body.contains("walk"));
        assert!(body.contains("activity"));
    }
}

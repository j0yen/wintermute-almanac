//! Property-based invariant tests.
//!
//! Read-only after scaffold. The edit-agent must NOT modify proptests.

use proptest::prelude::*;

proptest! {
    #[test]
    fn hhmm_roundtrip_valid(hour in 0u8..24u8, minute in 0u8..60u8) {
        let s = format!("{hour:02}:{minute:02}");
        let result = wintermute_almanac::parse_hhmm(&s);
        prop_assert!(result.is_ok(), "valid HH:MM should parse: {s}");
        let (h, m) = result.unwrap();
        prop_assert_eq!(h, hour);
        prop_assert_eq!(m, minute);
    }

    #[test]
    fn hhmm_rejects_invalid_hour(hour in 24u8..=255u8, minute in 0u8..60u8) {
        let s = format!("{hour:02}:{minute:02}");
        let result = wintermute_almanac::parse_hhmm(&s);
        prop_assert!(result.is_err(), "hour >= 24 should be rejected: {s}");
    }

    #[test]
    fn hhmm_rejects_invalid_minute(hour in 0u8..24u8, minute in 60u8..=255u8) {
        let s = format!("{hour:02}:{minute:02}");
        let result = wintermute_almanac::parse_hhmm(&s);
        prop_assert!(result.is_err(), "minute >= 60 should be rejected: {s}");
    }
}

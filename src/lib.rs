//! `wintermute-almanac` library — re-exports for integration tests and
//! downstream crates.

pub mod entry;
pub mod next;
pub mod store;

pub use entry::{parse_hhmm, parse_recurrence};

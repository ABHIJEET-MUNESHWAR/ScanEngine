//! Domain types for ScanEngine: instruments, ticks, the scanner condition DSL,
//! rules, and emitted signals. Pure data + validation, no I/O.

pub mod error;
pub mod rule;
pub mod signal;
pub mod stats;
pub mod tick;
pub mod units;

pub use error::{InvalidRule, InvalidTick};
pub use rule::{Comparator, Condition, Field, Rule, RuleId, Scope};
pub use signal::Signal;
pub use stats::ScanStats;
pub use tick::{InstrumentState, MarketTick};
pub use units::{InstrumentId, Price};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::rule::RuleId;
use crate::units::InstrumentId;

/// A signal emitted when a rule fires for an instrument (rising edge).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signal {
    /// Rule that fired.
    pub rule_id: RuleId,
    /// Human-readable rule name.
    pub rule_name: String,
    /// Instrument that triggered the rule.
    pub instrument: InstrumentId,
    /// Monotonic sequence number assigned by the engine.
    pub sequence: u64,
    /// Last price at trigger time.
    pub last_price: i64,
    /// Percentage change from open in basis points at trigger time.
    pub pct_change_bps: i64,
    /// Server time the signal was produced.
    pub triggered_at: DateTime<Utc>,
}

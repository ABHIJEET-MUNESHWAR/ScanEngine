use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::InvalidRule;
use crate::tick::InstrumentState;
use crate::units::InstrumentId;

/// Unique identifier for a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RuleId(pub Uuid);

impl RuleId {
    /// Generate a fresh random rule id.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RuleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RuleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A readable field of per-instrument state that a condition can test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    /// Last traded price in ticks.
    LastPrice,
    /// Session high in ticks.
    High,
    /// Session low in ticks.
    Low,
    /// Cumulative volume.
    Volume,
    /// Percentage change from open in basis points.
    PctChangeBps,
}

impl Field {
    /// Current value of the field.
    #[must_use]
    pub fn current(self, s: &InstrumentState) -> i64 {
        match self {
            Self::LastPrice => s.last,
            Self::High => s.high,
            Self::Low => s.low,
            Self::Volume => s.volume as i64,
            Self::PctChangeBps => s.pct_change_bps(),
        }
    }

    /// Previous value of the field (before the current tick).
    #[must_use]
    pub fn previous(self, s: &InstrumentState) -> i64 {
        match self {
            Self::LastPrice => s.prev_last,
            Self::High => s.high,
            Self::Low => s.low,
            Self::Volume => s.volume as i64,
            Self::PctChangeBps => s.prev_pct_change_bps(),
        }
    }
}

/// How a field value is compared against a threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparator {
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Gte,
    /// Less than.
    Lt,
    /// Less than or equal.
    Lte,
    /// Rising edge across the threshold (prev <= t and now > t).
    CrossAbove,
    /// Falling edge across the threshold (prev >= t and now < t).
    CrossBelow,
}

/// A single predicate over one field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Condition {
    /// Field being tested.
    pub field: Field,
    /// Comparison operator.
    pub comparator: Comparator,
    /// Threshold value (ticks, volume, or basis points depending on field).
    pub threshold: i64,
}

impl Condition {
    /// Evaluate this condition against instrument state.
    #[must_use]
    pub fn evaluate(&self, s: &InstrumentState) -> bool {
        let now = self.field.current(s);
        let prev = self.field.previous(s);
        match self.comparator {
            Comparator::Gt => now > self.threshold,
            Comparator::Gte => now >= self.threshold,
            Comparator::Lt => now < self.threshold,
            Comparator::Lte => now <= self.threshold,
            Comparator::CrossAbove => prev <= self.threshold && now > self.threshold,
            Comparator::CrossBelow => prev >= self.threshold && now < self.threshold,
        }
    }
}

/// Which instruments a rule applies to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// Applies to every instrument.
    Any,
    /// Applies only to one instrument.
    Instrument(InstrumentId),
}

/// A named rule: a conjunction (AND) of conditions over a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier.
    pub id: RuleId,
    /// Human-readable name.
    pub name: String,
    /// Instrument scope.
    pub scope: Scope,
    /// Conditions, all of which must hold for the rule to fire.
    pub conditions: Vec<Condition>,
}

impl Rule {
    /// Construct and validate a rule.
    ///
    /// # Errors
    /// Returns [`InvalidRule`] if the name is empty or there are no conditions.
    pub fn new(
        name: impl Into<String>,
        scope: Scope,
        conditions: Vec<Condition>,
    ) -> Result<Self, InvalidRule> {
        let name = name.into();
        if name.is_empty() {
            return Err(InvalidRule::EmptyName);
        }
        if conditions.is_empty() {
            return Err(InvalidRule::NoConditions);
        }
        Ok(Self {
            id: RuleId::new(),
            name,
            scope,
            conditions,
        })
    }

    /// True when this rule is in scope for `instrument`.
    #[must_use]
    pub fn applies_to(&self, instrument: &InstrumentId) -> bool {
        match &self.scope {
            Scope::Any => true,
            Scope::Instrument(id) => id == instrument,
        }
    }

    /// Evaluate all conditions (AND) against instrument state.
    #[must_use]
    pub fn matches(&self, s: &InstrumentState) -> bool {
        self.conditions.iter().all(|c| c.evaluate(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tick::MarketTick;
    use chrono::Utc;

    fn state(open: i64, prev: i64, last: i64) -> InstrumentState {
        let t = MarketTick {
            instrument: InstrumentId::new("X").unwrap(),
            last_price: open,
            volume: 0,
            exchange_time: Utc::now(),
        };
        let mut s = InstrumentState::from_first(&t);
        s.prev_last = prev;
        s.last = last;
        if last > s.high {
            s.high = last;
        }
        if last < s.low {
            s.low = last;
        }
        s
    }

    #[test]
    fn cross_above_is_rising_edge_only() {
        let c = Condition {
            field: Field::LastPrice,
            comparator: Comparator::CrossAbove,
            threshold: 100,
        };
        assert!(c.evaluate(&state(90, 99, 101))); // crossed up
        assert!(!c.evaluate(&state(90, 101, 102))); // already above
    }

    #[test]
    fn rule_is_conjunction() {
        let rule = Rule::new(
            "breakout",
            Scope::Any,
            vec![
                Condition {
                    field: Field::LastPrice,
                    comparator: Comparator::Gt,
                    threshold: 100,
                },
                Condition {
                    field: Field::PctChangeBps,
                    comparator: Comparator::Gte,
                    threshold: 200,
                },
            ],
        )
        .unwrap();
        // open 100, last 105 => +500 bps, price 105 > 100
        assert!(rule.matches(&state(100, 104, 105)));
        // last 101 => +100 bps < 200 bps
        assert!(!rule.matches(&state(100, 100, 101)));
    }

    #[test]
    fn rule_validation() {
        assert!(Rule::new("", Scope::Any, vec![]).is_err());
        assert!(Rule::new("n", Scope::Any, vec![]).is_err());
    }
}

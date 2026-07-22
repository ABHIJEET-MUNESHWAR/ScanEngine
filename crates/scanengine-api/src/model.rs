use async_graphql::{Enum, InputObject, SimpleObject};
use scanengine_core::{Explainer, HeuristicExplainer};
use scanengine_types::{
    Comparator, Condition, Field, InstrumentState, Rule, ScanStats, Scope, Signal,
};

/// GraphQL enum mirroring [`Field`].
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum FieldGql {
    /// Last traded price.
    LastPrice,
    /// Session high.
    High,
    /// Session low.
    Low,
    /// Cumulative volume.
    Volume,
    /// Percentage change from open in basis points.
    PctChangeBps,
}

impl From<FieldGql> for Field {
    fn from(f: FieldGql) -> Self {
        match f {
            FieldGql::LastPrice => Self::LastPrice,
            FieldGql::High => Self::High,
            FieldGql::Low => Self::Low,
            FieldGql::Volume => Self::Volume,
            FieldGql::PctChangeBps => Self::PctChangeBps,
        }
    }
}

/// GraphQL enum mirroring [`Comparator`].
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
#[graphql(rename_items = "SCREAMING_SNAKE_CASE")]
pub enum ComparatorGql {
    /// Greater than.
    Gt,
    /// Greater than or equal.
    Gte,
    /// Less than.
    Lt,
    /// Less than or equal.
    Lte,
    /// Rising edge across threshold.
    CrossAbove,
    /// Falling edge across threshold.
    CrossBelow,
}

impl From<ComparatorGql> for Comparator {
    fn from(c: ComparatorGql) -> Self {
        match c {
            ComparatorGql::Gt => Self::Gt,
            ComparatorGql::Gte => Self::Gte,
            ComparatorGql::Lt => Self::Lt,
            ComparatorGql::Lte => Self::Lte,
            ComparatorGql::CrossAbove => Self::CrossAbove,
            ComparatorGql::CrossBelow => Self::CrossBelow,
        }
    }
}

/// A single predicate input.
#[derive(InputObject)]
pub struct ConditionInput {
    /// Field to test.
    pub field: FieldGql,
    /// Comparison operator.
    pub comparator: ComparatorGql,
    /// Threshold value.
    pub threshold: i64,
}

impl From<ConditionInput> for Condition {
    fn from(c: ConditionInput) -> Self {
        Self {
            field: c.field.into(),
            comparator: c.comparator.into(),
            threshold: c.threshold,
        }
    }
}

/// Input to create a rule.
#[derive(InputObject)]
pub struct RuleInput {
    /// Human-readable name.
    pub name: String,
    /// Instrument scope; `null` means all instruments.
    pub instrument: Option<String>,
    /// Conditions (AND).
    pub conditions: Vec<ConditionInput>,
}

/// Input to publish a tick.
#[derive(InputObject)]
pub struct TickInput {
    /// Instrument identifier.
    pub instrument: String,
    /// Last traded price in ticks.
    pub last_price: i64,
    /// Cumulative volume.
    pub volume: u64,
}

/// GraphQL view of a condition.
#[derive(SimpleObject)]
pub struct ConditionObject {
    /// Field name.
    pub field: String,
    /// Comparator name.
    pub comparator: String,
    /// Threshold.
    pub threshold: i64,
}

/// GraphQL view of a rule.
#[derive(SimpleObject)]
pub struct RuleObject {
    /// Rule id.
    pub id: String,
    /// Name.
    pub name: String,
    /// Scope: instrument id or `*` for any.
    pub scope: String,
    /// Conditions.
    pub conditions: Vec<ConditionObject>,
}

impl From<Rule> for RuleObject {
    fn from(r: Rule) -> Self {
        let scope = match &r.scope {
            Scope::Any => "*".to_owned(),
            Scope::Instrument(id) => id.as_str().to_owned(),
        };
        Self {
            id: r.id.to_string(),
            name: r.name,
            scope,
            conditions: r
                .conditions
                .into_iter()
                .map(|c| ConditionObject {
                    field: format!("{:?}", c.field),
                    comparator: format!("{:?}", c.comparator),
                    threshold: c.threshold,
                })
                .collect(),
        }
    }
}

/// GraphQL view of an emitted signal, including a heuristic explanation.
#[derive(SimpleObject, Clone)]
pub struct SignalObject {
    /// Rule id that fired.
    pub rule_id: String,
    /// Rule name.
    pub rule_name: String,
    /// Instrument that triggered.
    pub instrument: String,
    /// Sequence number.
    pub sequence: u64,
    /// Last price at trigger.
    pub last_price: i64,
    /// Percentage change from open in basis points.
    pub pct_change_bps: i64,
    /// Natural-language explanation.
    pub explanation: String,
    /// RFC3339 trigger time.
    pub triggered_at: String,
}

impl From<Signal> for SignalObject {
    fn from(s: Signal) -> Self {
        let explanation = HeuristicExplainer::new().explain(&s);
        Self {
            rule_id: s.rule_id.to_string(),
            rule_name: s.rule_name.clone(),
            instrument: s.instrument.as_str().to_owned(),
            sequence: s.sequence,
            last_price: s.last_price,
            pct_change_bps: s.pct_change_bps,
            explanation,
            triggered_at: s.triggered_at.to_rfc3339(),
        }
    }
}

/// GraphQL view of instrument state.
#[derive(SimpleObject)]
pub struct StateObject {
    /// Session open.
    pub open: i64,
    /// Last price.
    pub last: i64,
    /// Session high.
    pub high: i64,
    /// Session low.
    pub low: i64,
    /// Volume.
    pub volume: u64,
    /// Percentage change from open in basis points.
    pub pct_change_bps: i64,
}

impl From<InstrumentState> for StateObject {
    fn from(s: InstrumentState) -> Self {
        Self {
            open: s.open,
            last: s.last,
            high: s.high,
            low: s.low,
            volume: s.volume,
            pct_change_bps: s.pct_change_bps(),
        }
    }
}

/// GraphQL view of engine statistics.
#[derive(SimpleObject)]
pub struct StatsObject {
    /// Ticks processed.
    pub ticks_processed: u64,
    /// Ticks rejected.
    pub ticks_rejected: u64,
    /// Total condition evaluations.
    pub evaluations: u64,
    /// Signals emitted.
    pub signals_emitted: u64,
    /// Registered rules.
    pub rules: u64,
    /// Tracked instruments.
    pub instruments: u64,
    /// Average evaluations per tick.
    pub evaluations_per_tick: f64,
}

impl From<ScanStats> for StatsObject {
    fn from(s: ScanStats) -> Self {
        let evaluations_per_tick = s.evaluations_per_tick();
        Self {
            ticks_processed: s.ticks_processed,
            ticks_rejected: s.ticks_rejected,
            evaluations: s.evaluations,
            signals_emitted: s.signals_emitted,
            rules: s.rules,
            instruments: s.instruments,
            evaluations_per_tick,
        }
    }
}

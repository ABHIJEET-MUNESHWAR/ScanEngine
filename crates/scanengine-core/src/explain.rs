use scanengine_types::Signal;

/// Produces a human-readable explanation for an emitted signal.
///
/// This is an intentionally deterministic, dependency-free "agentic" component:
/// it summarizes *why* a rule fired in natural language. The trait leaves room
/// for a future LLM-backed implementation, while [`HeuristicExplainer`] gives a
/// reliable, offline default (the recommended fallback for AI features).
pub trait Explainer: Send + Sync {
    /// Explain a signal in one human-readable sentence.
    fn explain(&self, signal: &Signal) -> String;
}

/// A rule-based explainer that turns signal fields into prose.
#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicExplainer;

impl HeuristicExplainer {
    /// Create a new explainer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    fn momentum(bps: i64) -> &'static str {
        match bps {
            b if b >= 500 => "a strong upward move",
            b if b >= 100 => "a moderate gain",
            b if b > 0 => "a mild uptick",
            0 => "a flat session",
            b if b > -100 => "a mild dip",
            b if b > -500 => "a moderate decline",
            _ => "a sharp sell-off",
        }
    }
}

impl Explainer for HeuristicExplainer {
    fn explain(&self, signal: &Signal) -> String {
        let pct = signal.pct_change_bps as f64 / 100.0;
        format!(
            "{} triggered on {} at price {} ({:+.2}% from open) — {}.",
            signal.rule_name,
            signal.instrument.as_str(),
            signal.last_price,
            pct,
            Self::momentum(signal.pct_change_bps),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use scanengine_types::{InstrumentId, RuleId};

    fn signal(bps: i64) -> Signal {
        Signal {
            rule_id: RuleId::new(),
            rule_name: "Breakout".to_owned(),
            instrument: InstrumentId::new("NSE:TCS").unwrap(),
            sequence: 1,
            last_price: 105,
            pct_change_bps: bps,
            triggered_at: Utc::now(),
        }
    }

    #[test]
    fn explains_upward_momentum() {
        let text = HeuristicExplainer::new().explain(&signal(600));
        assert!(text.contains("Breakout"));
        assert!(text.contains("NSE:TCS"));
        assert!(text.contains("strong upward move"));
    }

    #[test]
    fn explains_sell_off() {
        let text = HeuristicExplainer::new().explain(&signal(-800));
        assert!(text.contains("sharp sell-off"));
    }
}

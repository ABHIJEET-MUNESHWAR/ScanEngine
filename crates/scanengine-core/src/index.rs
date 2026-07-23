use std::collections::HashMap;
use std::sync::Arc;

use scanengine_types::{InstrumentId, Rule, RuleId, Scope};

/// An in-memory, read-optimized index of rules for the hot evaluation path.
///
/// Rules are grouped so that a tick for instrument `X` only needs to look at
/// rules scoped to `X` plus the `Any`-scoped rules — the incremental
/// (dirty-set) optimization that keeps per-tick work proportional to the
/// rules that could possibly fire, not the entire rule base.
#[derive(Default)]
pub struct RuleIndex {
    by_instrument: HashMap<InstrumentId, Vec<Arc<Rule>>>,
    any: Vec<Arc<Rule>>,
}

impl RuleIndex {
    /// Create an empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a rule into the index.
    pub fn insert(&mut self, rule: Arc<Rule>) {
        match &rule.scope {
            Scope::Any => self.any.push(rule),
            Scope::Instrument(id) => self.by_instrument.entry(id.clone()).or_default().push(rule),
        }
    }

    /// Remove a rule by id, returning whether it was present.
    pub fn remove(&mut self, id: RuleId) -> bool {
        let before = self.len();
        self.any.retain(|r| r.id != id);
        self.by_instrument.retain(|_, v| {
            v.retain(|r| r.id != id);
            !v.is_empty()
        });
        self.len() != before
    }

    /// Rules that could fire for `instrument` (instrument-scoped + `Any`).
    #[must_use]
    pub fn candidates(&self, instrument: &InstrumentId) -> Vec<Arc<Rule>> {
        let mut out = self.any.clone();
        if let Some(v) = self.by_instrument.get(instrument) {
            out.extend(v.iter().cloned());
        }
        out
    }

    /// Visit every candidate rule for `instrument` in place, without allocating
    /// or cloning the candidate set. This is the hot evaluation path: a tick
    /// only touches its instrument-scoped rules plus the `Any`-scoped rules.
    pub fn for_each_candidate<F: FnMut(&Rule)>(&self, instrument: &InstrumentId, mut f: F) {
        for rule in &self.any {
            f(rule);
        }
        if let Some(v) = self.by_instrument.get(instrument) {
            for rule in v {
                f(rule);
            }
        }
    }

    /// Total number of indexed rules.
    #[must_use]
    pub fn len(&self) -> usize {
        self.any.len() + self.by_instrument.values().map(Vec::len).sum::<usize>()
    }

    /// Whether the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scanengine_types::{Comparator, Condition, Field};

    fn rule(scope: Scope) -> Arc<Rule> {
        Arc::new(
            Rule::new(
                "r",
                scope,
                vec![Condition {
                    field: Field::LastPrice,
                    comparator: Comparator::Gt,
                    threshold: 1,
                }],
            )
            .unwrap(),
        )
    }

    #[test]
    fn candidates_include_any_and_scoped() {
        let tcs = InstrumentId::new("NSE:TCS").unwrap();
        let infy = InstrumentId::new("NSE:INFY").unwrap();
        let mut idx = RuleIndex::new();
        idx.insert(rule(Scope::Any));
        idx.insert(rule(Scope::Instrument(tcs.clone())));
        idx.insert(rule(Scope::Instrument(infy.clone())));
        assert_eq!(idx.len(), 3);
        assert_eq!(idx.candidates(&tcs).len(), 2); // any + tcs
        assert_eq!(idx.candidates(&infy).len(), 2); // any + infy
    }

    #[test]
    fn remove_prunes() {
        let mut idx = RuleIndex::new();
        let r = rule(Scope::Any);
        let id = r.id;
        idx.insert(r);
        assert!(idx.remove(id));
        assert!(!idx.remove(id));
        assert!(idx.is_empty());
    }
}

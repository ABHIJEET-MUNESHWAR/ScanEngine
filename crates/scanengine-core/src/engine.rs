use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use chrono::Utc;
use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use scanengine_resilience::RateLimiter;
use scanengine_types::{
    Condition, InstrumentId, InstrumentState, MarketTick, Rule, RuleId, ScanStats, Scope, Signal,
};

use crate::config::ScanConfig;
use crate::error::CoreError;
use crate::index::RuleIndex;
use crate::ports::{RuleStore, SignalBus, SignalStream};

/// The incremental CEP engine: maintains per-instrument state, evaluates only
/// the rules relevant to each tick (dirty set), and emits signals on rising
/// edges.
///
/// Generic over its [`RuleStore`] and [`SignalBus`] ports.
pub struct ScanEngine<R, B>
where
    R: RuleStore,
    B: SignalBus,
{
    config: ScanConfig,
    store: Arc<R>,
    bus: Arc<B>,
    index: RwLock<RuleIndex>,
    states: DashMap<InstrumentId, InstrumentState>,
    firing: DashMap<InstrumentId, HashSet<RuleId>>,
    limiter: RateLimiter,
    sequence: AtomicU64,
    stats: Mutex<ScanStats>,
}

impl<R, B> ScanEngine<R, B>
where
    R: RuleStore,
    B: SignalBus,
{
    /// Assemble an engine from its config and ports.
    #[must_use]
    pub fn new(config: ScanConfig, store: Arc<R>, bus: Arc<B>) -> Self {
        let limiter = RateLimiter::new(config.ingest_burst, config.ingest_refill_per_sec);
        Self {
            config,
            store,
            bus,
            index: RwLock::new(RuleIndex::new()),
            states: DashMap::new(),
            firing: DashMap::new(),
            limiter,
            sequence: AtomicU64::new(0),
            stats: Mutex::new(ScanStats::default()),
        }
    }

    /// Register and persist a new rule, returning its id.
    ///
    /// # Errors
    /// Returns [`CoreError::InvalidRule`] if the rule is malformed, or
    /// [`CoreError::Port`] if persistence fails.
    pub async fn add_rule(
        &self,
        name: impl Into<String>,
        scope: Scope,
        conditions: Vec<Condition>,
    ) -> Result<RuleId, CoreError> {
        let rule = Rule::new(name, scope, conditions)?;
        let id = rule.id;
        self.store.add(rule.clone()).await?;
        self.index.write().insert(Arc::new(rule));
        {
            let mut s = self.stats.lock();
            s.rules += 1;
        }
        Ok(id)
    }

    /// Remove a rule by id.
    ///
    /// # Errors
    /// Returns [`CoreError::Port`] if the store fails.
    pub async fn remove_rule(&self, id: RuleId) -> Result<bool, CoreError> {
        let removed = self.store.remove(id).await?;
        if self.index.write().remove(id) && removed {
            let mut s = self.stats.lock();
            s.rules = s.rules.saturating_sub(1);
        }
        Ok(removed)
    }

    fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::Relaxed) + 1
    }

    /// Process a single tick: update state, evaluate the dirty set, and emit
    /// any newly-firing signals.
    ///
    /// # Errors
    /// - [`CoreError::RateLimited`] on admission control rejection.
    /// - [`CoreError::InvalidTick`] on validation failure.
    /// - [`CoreError::CapacityExceeded`] when a new instrument exceeds the cap.
    /// - [`CoreError::Port`] when the bus fails.
    pub async fn process(&self, tick: MarketTick) -> Result<Vec<Signal>, CoreError> {
        if self.limiter.try_acquire().is_err() {
            metrics::counter!("scanengine_rate_limited_total").increment(1);
            self.stats.lock().ticks_rejected += 1;
            return Err(CoreError::RateLimited);
        }
        if let Err(e) = tick.validate() {
            self.stats.lock().ticks_rejected += 1;
            return Err(CoreError::InvalidTick(e));
        }

        let instrument = tick.instrument.clone();
        let is_new = !self.states.contains_key(&instrument);
        if is_new && self.states.len() >= self.config.max_instruments {
            return Err(CoreError::CapacityExceeded(self.config.max_instruments));
        }

        let state = {
            let mut entry = self
                .states
                .entry(instrument.clone())
                .or_insert_with(|| InstrumentState::from_first(&tick));
            if !is_new {
                entry.apply(&tick);
            }
            *entry
        };

        // Evaluate the candidate rules under the read lock in a scoped block so
        // the guard is released before any `.await` (bus publish). No candidate
        // set is cloned: rules are visited in place.
        let mut evaluations = 0u64;
        let signals = {
            let index = self.index.read();
            let mut signals = Vec::new();
            // One firing-set lookup per tick, keyed by instrument. Rule ids are
            // `Copy`, so per-candidate membership updates allocate nothing — the
            // previous design cloned the instrument id into a composite key for
            // every candidate rule (up to thousands of allocations per tick).
            let mut firing = self.firing.entry(instrument.clone()).or_default();
            index.for_each_candidate(&instrument, |rule| {
                evaluations += rule.conditions.len() as u64;
                if rule.matches(&state) {
                    // Rising edge: only emit when transitioning from not-firing.
                    if firing.insert(rule.id) {
                        signals.push(Signal {
                            rule_id: rule.id,
                            rule_name: rule.name.clone(),
                            instrument: instrument.clone(),
                            sequence: self.next_sequence(),
                            last_price: state.last,
                            pct_change_bps: state.pct_change_bps(),
                            triggered_at: Utc::now(),
                        });
                    }
                } else {
                    firing.remove(&rule.id);
                }
            });
            signals
        };

        for signal in &signals {
            self.bus.publish(signal.clone()).await?;
        }

        {
            let mut s = self.stats.lock();
            s.ticks_processed += 1;
            s.evaluations += evaluations;
            s.signals_emitted += signals.len() as u64;
            s.instruments = self.states.len() as u64;
        }
        metrics::counter!("scanengine_ticks_processed_total").increment(1);
        if !signals.is_empty() {
            metrics::counter!("scanengine_signals_emitted_total").increment(signals.len() as u64);
        }

        Ok(signals)
    }

    /// Current state for an instrument, if tracked.
    #[must_use]
    pub fn state(&self, instrument: &InstrumentId) -> Option<InstrumentState> {
        self.states.get(instrument).map(|e| *e.value())
    }

    /// All registered rules.
    ///
    /// # Errors
    /// Returns [`CoreError::Port`] if the store fails.
    pub async fn rules(&self) -> Result<Vec<Rule>, CoreError> {
        Ok(self.store.all().await?)
    }

    /// Subscribe to the signal stream, optionally filtered by rule id.
    #[must_use]
    pub fn subscribe(&self, rules: Vec<RuleId>) -> SignalStream {
        self.bus.subscribe(rules)
    }

    /// Snapshot of current statistics.
    #[must_use]
    pub fn stats(&self) -> ScanStats {
        *self.stats.lock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{MockRuleStore, MockSignalBus};
    use scanengine_types::{Comparator, Field};

    fn tick(inst: &str, px: i64) -> MarketTick {
        MarketTick {
            instrument: InstrumentId::new(inst).unwrap(),
            last_price: px,
            volume: 100,
            exchange_time: Utc::now(),
        }
    }

    fn engine(
        store: MockRuleStore,
        bus: MockSignalBus,
    ) -> ScanEngine<MockRuleStore, MockSignalBus> {
        ScanEngine::new(ScanConfig::default(), Arc::new(store), Arc::new(bus))
    }

    #[tokio::test]
    async fn fires_once_on_rising_edge_then_resets() {
        let mut store = MockRuleStore::new();
        store.expect_add().returning(|_| Ok(()));
        let mut bus = MockSignalBus::new();
        bus.expect_publish().returning(|_| Ok(()));

        let eng = engine(store, bus);
        eng.add_rule(
            "above-105",
            Scope::Any,
            vec![Condition {
                field: Field::LastPrice,
                comparator: Comparator::Gt,
                threshold: 105,
            }],
        )
        .await
        .unwrap();

        // Seed at 100 (below threshold) -> no signal.
        assert!(eng.process(tick("NSE:TCS", 100)).await.unwrap().is_empty());
        // Cross to 110 -> exactly one signal (rising edge).
        assert_eq!(eng.process(tick("NSE:TCS", 110)).await.unwrap().len(), 1);
        // Stay above -> no new signal (already firing).
        assert!(eng.process(tick("NSE:TCS", 111)).await.unwrap().is_empty());
        // Drop below -> resets firing state.
        assert!(eng.process(tick("NSE:TCS", 100)).await.unwrap().is_empty());
        // Cross again -> fires again.
        assert_eq!(eng.process(tick("NSE:TCS", 120)).await.unwrap().len(), 1);

        assert_eq!(eng.stats().signals_emitted, 2);
    }

    #[tokio::test]
    async fn rejects_negative_price() {
        let store = MockRuleStore::new();
        let bus = MockSignalBus::new();
        let eng = engine(store, bus);
        let err = eng.process(tick("NSE:TCS", -1)).await.unwrap_err();
        assert!(matches!(err, CoreError::InvalidTick(_)));
        assert_eq!(eng.stats().ticks_rejected, 1);
    }

    #[tokio::test]
    async fn scoped_rule_ignores_other_instruments() {
        let mut store = MockRuleStore::new();
        store.expect_add().returning(|_| Ok(()));
        let mut bus = MockSignalBus::new();
        bus.expect_publish().returning(|_| Ok(()));
        let eng = engine(store, bus);
        eng.add_rule(
            "tcs-only",
            Scope::Instrument(InstrumentId::new("NSE:TCS").unwrap()),
            vec![Condition {
                field: Field::LastPrice,
                comparator: Comparator::Gt,
                threshold: 50,
            }],
        )
        .await
        .unwrap();
        // INFY is out of scope -> no signal even though price > 50.
        assert!(eng.process(tick("NSE:INFY", 100)).await.unwrap().is_empty());
        // TCS in scope -> fires.
        assert_eq!(eng.process(tick("NSE:TCS", 100)).await.unwrap().len(), 1);
    }
}

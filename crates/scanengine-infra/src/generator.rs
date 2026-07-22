use chrono::Utc;
use scanengine_types::{InstrumentId, MarketTick};

/// A deterministic market-tick generator (LCG-based, no `rand`) for demos,
/// replay, and load tests. Prices oscillate so that threshold-cross rules fire
/// repeatably.
pub struct TickGenerator {
    instruments: Vec<InstrumentId>,
    state: u64,
    cursor: usize,
    step: u64,
}

impl TickGenerator {
    /// Build a generator over `count` synthetic instruments seeded by `seed`.
    #[must_use]
    pub fn new(count: usize, seed: u64) -> Self {
        let instruments = (0..count.max(1))
            .map(|i| {
                InstrumentId::new(format!("SIM:{i:06}"))
                    .expect("synthetic instrument id is always valid")
            })
            .collect();
        Self {
            instruments,
            state: seed | 1,
            cursor: 0,
            step: 0,
        }
    }

    fn next_rand(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    /// Produce the next deterministic tick, cycling through instruments and
    /// oscillating each instrument's price around its own stable base so rules
    /// trigger periodically (rather than on every tick).
    pub fn next_tick(&mut self) -> MarketTick {
        let idx = self.cursor % self.instruments.len();
        let instrument = self.instruments[idx].clone();
        self.cursor += 1;
        self.step += 1;

        let r = self.next_rand();
        // Stable base per instrument (does not change tick-to-tick).
        let base = 100 + (idx as i64 % 400); // 100..=499
                                             // Slow triangular oscillation driven by how many times this instrument
                                             // has been visited, so every instrument advances through its own cycle.
        let cycle = (self.cursor / self.instruments.len()) as i64;
        let phase = (cycle + idx as i64) % 200;
        let osc = if phase < 100 { phase } else { 200 - phase } - 50;
        let last = (base + osc).max(1);
        MarketTick {
            instrument,
            last_price: last,
            volume: (r >> 20) % 1_000_000,
            exchange_time: Utc::now(),
        }
    }

    /// Number of distinct instruments this generator cycles through.
    #[must_use]
    pub fn instrument_count(&self) -> usize {
        self.instruments.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_deterministic_for_a_given_seed() {
        let mut a = TickGenerator::new(8, 42);
        let mut b = TickGenerator::new(8, 42);
        for _ in 0..200 {
            let ta = a.next_tick();
            let tb = b.next_tick();
            assert_eq!(ta.instrument, tb.instrument);
            assert_eq!(ta.last_price, tb.last_price);
        }
    }

    #[test]
    fn produces_valid_ticks() {
        let mut g = TickGenerator::new(16, 7);
        for _ in 0..1_000 {
            assert!(g.next_tick().validate().is_ok());
        }
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::InvalidTick;
use crate::units::InstrumentId;

/// A market update for a single instrument, the unit of evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketTick {
    /// Instrument the update refers to.
    pub instrument: InstrumentId,
    /// Last traded price in ticks (non-negative).
    pub last_price: i64,
    /// Cumulative session volume.
    pub volume: u64,
    /// Exchange timestamp.
    pub exchange_time: DateTime<Utc>,
}

impl MarketTick {
    /// Validate the tick's field invariants.
    ///
    /// # Errors
    /// Returns [`InvalidTick::NegativePrice`] for a negative price.
    pub fn validate(&self) -> Result<(), InvalidTick> {
        if self.last_price < 0 {
            return Err(InvalidTick::NegativePrice(self.last_price));
        }
        Ok(())
    }
}

/// Rolling per-instrument state maintained by the engine and read by
/// conditions. Holds the previous last price so edge (cross) comparators are
/// deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentState {
    /// Session open price.
    pub open: i64,
    /// Most recent last price.
    pub last: i64,
    /// Previous last price (before the current tick).
    pub prev_last: i64,
    /// Session high.
    pub high: i64,
    /// Session low.
    pub low: i64,
    /// Latest cumulative volume.
    pub volume: u64,
    /// Number of ticks applied.
    pub updates: u64,
}

impl InstrumentState {
    /// Seed state from the first tick of the session.
    #[must_use]
    pub fn from_first(tick: &MarketTick) -> Self {
        Self {
            open: tick.last_price,
            last: tick.last_price,
            prev_last: tick.last_price,
            high: tick.last_price,
            low: tick.last_price,
            volume: tick.volume,
            updates: 1,
        }
    }

    /// Apply a subsequent tick, advancing OHLC and the previous-last marker.
    pub fn apply(&mut self, tick: &MarketTick) {
        self.prev_last = self.last;
        self.last = tick.last_price;
        if tick.last_price > self.high {
            self.high = tick.last_price;
        }
        if tick.last_price < self.low {
            self.low = tick.last_price;
        }
        self.volume = tick.volume;
        self.updates += 1;
    }

    /// Percentage change from open, expressed in basis points (1% = 100 bps).
    #[must_use]
    pub fn pct_change_bps(&self) -> i64 {
        if self.open == 0 {
            return 0;
        }
        (self.last - self.open) * 10_000 / self.open
    }

    /// Previous percentage change from open in basis points.
    #[must_use]
    pub fn prev_pct_change_bps(&self) -> i64 {
        if self.open == 0 {
            return 0;
        }
        (self.prev_last - self.open) * 10_000 / self.open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(px: i64, vol: u64) -> MarketTick {
        MarketTick {
            instrument: InstrumentId::new("NSE:TCS").unwrap(),
            last_price: px,
            volume: vol,
            exchange_time: Utc::now(),
        }
    }

    #[test]
    fn state_tracks_ohlc_and_prev() {
        let mut s = InstrumentState::from_first(&tick(100, 10));
        s.apply(&tick(120, 20));
        s.apply(&tick(90, 30));
        assert_eq!(s.high, 120);
        assert_eq!(s.low, 90);
        assert_eq!(s.open, 100);
        assert_eq!(s.last, 90);
        assert_eq!(s.prev_last, 120);
    }

    #[test]
    fn pct_change_in_bps() {
        let mut s = InstrumentState::from_first(&tick(100, 0));
        s.apply(&tick(105, 0));
        assert_eq!(s.pct_change_bps(), 500);
    }
}

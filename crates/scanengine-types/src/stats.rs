use serde::{Deserialize, Serialize};

/// Aggregate counters describing engine activity.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanStats {
    /// Ticks accepted and evaluated.
    pub ticks_processed: u64,
    /// Ticks rejected by validation.
    pub ticks_rejected: u64,
    /// Total condition evaluations performed.
    pub evaluations: u64,
    /// Signals emitted (rising edges).
    pub signals_emitted: u64,
    /// Rules currently registered.
    pub rules: u64,
    /// Instruments currently tracked.
    pub instruments: u64,
}

impl ScanStats {
    /// Average number of condition evaluations per processed tick.
    #[must_use]
    pub fn evaluations_per_tick(&self) -> f64 {
        if self.ticks_processed == 0 {
            0.0
        } else {
            self.evaluations as f64 / self.ticks_processed as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluations_per_tick_is_safe_when_empty() {
        assert_eq!(ScanStats::default().evaluations_per_tick(), 0.0);
    }

    #[test]
    fn evaluations_per_tick_computes() {
        let s = ScanStats {
            ticks_processed: 4,
            evaluations: 20,
            ..ScanStats::default()
        };
        assert_eq!(s.evaluations_per_tick(), 5.0);
    }
}

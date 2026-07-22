/// Tuning knobs for the scan engine.
#[derive(Debug, Clone, Copy)]
pub struct ScanConfig {
    /// Maximum number of distinct instruments tracked.
    pub max_instruments: usize,
    /// Token-bucket burst capacity for ingest admission control.
    pub ingest_burst: f64,
    /// Sustained ingest rate (ticks/sec) for admission control.
    pub ingest_refill_per_sec: f64,
    /// Per-subscriber signal buffer depth.
    pub signal_buffer: usize,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_instruments: 100_000,
            ingest_burst: 1_000_000.0,
            ingest_refill_per_sec: 1_000_000.0,
            signal_buffer: 4_096,
        }
    }
}

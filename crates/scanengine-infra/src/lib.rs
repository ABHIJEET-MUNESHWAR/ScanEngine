//! Infrastructure adapters for ScanEngine: in-memory rule store, a
//! broadcast-based signal bus, and a deterministic tick generator.

pub mod bus;
pub mod generator;
pub mod store;

pub use bus::BroadcastSignalBus;
pub use generator::TickGenerator;
pub use store::InMemoryRuleStore;

//! Core domain logic for ScanEngine: an incremental complex-event-processing
//! (CEP) rules engine that evaluates thousands of live scanner conditions and
//! emits signals on rising edges.

pub mod config;
pub mod engine;
pub mod error;
pub mod explain;
pub mod index;
pub mod ports;

pub use config::ScanConfig;
pub use engine::ScanEngine;
pub use error::{CoreError, PortError};
pub use explain::{Explainer, HeuristicExplainer};
pub use index::RuleIndex;
pub use ports::{RuleStore, SignalBus, SignalStream};

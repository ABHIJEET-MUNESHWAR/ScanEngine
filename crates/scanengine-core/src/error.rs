use scanengine_types::{InvalidRule, InvalidTick};
use thiserror::Error;

/// Errors returned by infra adapters implementing the core ports.
#[derive(Debug, Error)]
pub enum PortError {
    /// The backing store or bus is unavailable.
    #[error("port unavailable: {0}")]
    Unavailable(String),
    /// The requested entity was not found.
    #[error("not found: {0}")]
    NotFound(String),
    /// A transient error the caller may retry.
    #[error("transient port error: {0}")]
    Transient(String),
}

impl PortError {
    /// True when the failure is worth retrying.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Transient(_) | Self::Unavailable(_))
    }
}

/// Errors surfaced by the scan engine.
#[derive(Debug, Error)]
pub enum CoreError {
    /// The incoming tick failed validation.
    #[error("invalid tick: {0}")]
    InvalidTick(#[from] InvalidTick),
    /// A submitted rule was invalid.
    #[error("invalid rule: {0}")]
    InvalidRule(#[from] InvalidRule),
    /// Ingest was rejected by admission control.
    #[error("ingest rate limit exceeded")]
    RateLimited,
    /// The instrument capacity was exceeded.
    #[error("instrument capacity {0} exceeded")]
    CapacityExceeded(usize),
    /// A downstream port failed.
    #[error(transparent)]
    Port(#[from] PortError),
}

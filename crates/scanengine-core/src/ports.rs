use async_trait::async_trait;
use futures::stream::BoxStream;
use scanengine_types::{Rule, RuleId, Signal};

use crate::error::PortError;

/// A live stream of signals delivered to one subscriber.
pub type SignalStream = BoxStream<'static, Signal>;

/// Durable-ish store of rule definitions.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RuleStore: Send + Sync {
    /// Persist a new rule.
    async fn add(&self, rule: Rule) -> Result<(), PortError>;

    /// Remove a rule by id, returning whether it existed.
    async fn remove(&self, id: RuleId) -> Result<bool, PortError>;

    /// Fetch all rules.
    async fn all(&self) -> Result<Vec<Rule>, PortError>;

    /// Number of stored rules.
    async fn count(&self) -> Result<usize, PortError>;
}

/// Fan-out hub for emitted signals.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SignalBus: Send + Sync {
    /// Publish a signal to all subscribers.
    async fn publish(&self, signal: Signal) -> Result<(), PortError>;

    /// Register a subscriber for signals (optionally filtered by rule).
    fn subscribe(&self, rules: Vec<RuleId>) -> SignalStream;

    /// Current number of live subscribers.
    fn subscriber_count(&self) -> usize;
}

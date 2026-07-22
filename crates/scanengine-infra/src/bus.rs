use std::collections::HashSet;

use async_trait::async_trait;
use futures::stream::StreamExt;
use scanengine_core::{PortError, SignalBus, SignalStream};
use scanengine_types::{RuleId, Signal};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

/// A fan-out bus for signals built on a Tokio broadcast channel.
pub struct BroadcastSignalBus {
    tx: broadcast::Sender<Signal>,
}

impl BroadcastSignalBus {
    /// Create a bus with a per-subscriber buffer of `capacity` signals.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity.max(1));
        Self { tx }
    }
}

#[async_trait]
impl SignalBus for BroadcastSignalBus {
    async fn publish(&self, signal: Signal) -> Result<(), PortError> {
        let _ = self.tx.send(signal);
        Ok(())
    }

    fn subscribe(&self, rules: Vec<RuleId>) -> SignalStream {
        let filter: HashSet<RuleId> = rules.into_iter().collect();
        let stream = BroadcastStream::new(self.tx.subscribe()).filter_map(move |res| {
            let filter = filter.clone();
            async move {
                match res {
                    Ok(sig) if filter.is_empty() || filter.contains(&sig.rule_id) => Some(sig),
                    _ => None,
                }
            }
        });
        stream.boxed()
    }

    fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use scanengine_types::InstrumentId;

    fn signal(rule_id: RuleId) -> Signal {
        Signal {
            rule_id,
            rule_name: "r".to_owned(),
            instrument: InstrumentId::new("NSE:TCS").unwrap(),
            sequence: 1,
            last_price: 100,
            pct_change_bps: 0,
            triggered_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn delivers_matching_rule_only() {
        let bus = BroadcastSignalBus::new(16);
        let wanted = RuleId::new();
        let other = RuleId::new();
        let mut sub = bus.subscribe(vec![wanted]);
        assert_eq!(bus.subscriber_count(), 1);

        bus.publish(signal(other)).await.unwrap();
        bus.publish(signal(wanted)).await.unwrap();

        let got = sub.next().await.unwrap();
        assert_eq!(got.rule_id, wanted);
    }
}

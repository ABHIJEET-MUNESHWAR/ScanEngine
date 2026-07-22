use async_trait::async_trait;
use dashmap::DashMap;
use scanengine_core::{PortError, RuleStore};
use scanengine_types::{Rule, RuleId};

/// An in-memory rule store backed by a sharded [`DashMap`].
#[derive(Default)]
pub struct InMemoryRuleStore {
    rules: DashMap<RuleId, Rule>,
}

impl InMemoryRuleStore {
    /// Create an empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl RuleStore for InMemoryRuleStore {
    async fn add(&self, rule: Rule) -> Result<(), PortError> {
        self.rules.insert(rule.id, rule);
        Ok(())
    }

    async fn remove(&self, id: RuleId) -> Result<bool, PortError> {
        Ok(self.rules.remove(&id).is_some())
    }

    async fn all(&self) -> Result<Vec<Rule>, PortError> {
        Ok(self.rules.iter().map(|e| e.value().clone()).collect())
    }

    async fn count(&self) -> Result<usize, PortError> {
        Ok(self.rules.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scanengine_types::{Comparator, Condition, Field, Scope};

    fn rule() -> Rule {
        Rule::new(
            "r",
            Scope::Any,
            vec![Condition {
                field: Field::LastPrice,
                comparator: Comparator::Gt,
                threshold: 1,
            }],
        )
        .unwrap()
    }

    #[tokio::test]
    async fn add_remove_count() {
        let store = InMemoryRuleStore::new();
        let r = rule();
        let id = r.id;
        store.add(r).await.unwrap();
        assert_eq!(store.count().await.unwrap(), 1);
        assert_eq!(store.all().await.unwrap().len(), 1);
        assert!(store.remove(id).await.unwrap());
        assert!(!store.remove(id).await.unwrap());
        assert_eq!(store.count().await.unwrap(), 0);
    }
}

use std::sync::Arc;

use async_graphql::{Context, Error, Object, Schema, Subscription};
use chrono::Utc;
use futures::stream::{Stream, StreamExt};
use scanengine_core::ScanEngine;
use scanengine_infra::{BroadcastSignalBus, InMemoryRuleStore};
use scanengine_types::{Condition, InstrumentId, MarketTick, RuleId, Scope};
use uuid::Uuid;

use crate::model::{RuleInput, RuleObject, SignalObject, StateObject, StatsObject, TickInput};

/// Concrete engine type wired into the GraphQL context.
pub type AppEngine = Arc<ScanEngine<InMemoryRuleStore, BroadcastSignalBus>>;

/// The composed GraphQL schema type.
pub type ScanSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

fn to_err<E: std::fmt::Display>(e: E) -> Error {
    Error::new(e.to_string())
}

fn parse_rule_id(s: &str) -> Result<RuleId, Error> {
    Uuid::parse_str(s).map(RuleId).map_err(to_err)
}

/// Read-only queries.
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// All registered rules.
    async fn rules(&self, ctx: &Context<'_>) -> Result<Vec<RuleObject>, Error> {
        let engine = ctx.data::<AppEngine>()?;
        let rules = engine.rules().await.map_err(to_err)?;
        Ok(rules.into_iter().map(RuleObject::from).collect())
    }

    /// Current state for one instrument.
    async fn state(
        &self,
        ctx: &Context<'_>,
        instrument: String,
    ) -> Result<Option<StateObject>, Error> {
        let engine = ctx.data::<AppEngine>()?;
        let id = InstrumentId::new(instrument).map_err(to_err)?;
        Ok(engine.state(&id).map(StateObject::from))
    }

    /// Engine-wide statistics.
    async fn stats(&self, ctx: &Context<'_>) -> Result<StatsObject, Error> {
        let engine = ctx.data::<AppEngine>()?;
        Ok(StatsObject::from(engine.stats()))
    }
}

/// Mutations.
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Register a new scanner rule; returns its id.
    async fn add_rule(&self, ctx: &Context<'_>, input: RuleInput) -> Result<String, Error> {
        let engine = ctx.data::<AppEngine>()?;
        let scope = match input.instrument {
            Some(inst) => Scope::Instrument(InstrumentId::new(inst).map_err(to_err)?),
            None => Scope::Any,
        };
        let conditions: Vec<Condition> = input.conditions.into_iter().map(Into::into).collect();
        let id = engine
            .add_rule(input.name, scope, conditions)
            .await
            .map_err(to_err)?;
        Ok(id.to_string())
    }

    /// Remove a rule by id.
    async fn remove_rule(&self, ctx: &Context<'_>, id: String) -> Result<bool, Error> {
        let engine = ctx.data::<AppEngine>()?;
        let rule_id = parse_rule_id(&id)?;
        engine.remove_rule(rule_id).await.map_err(to_err)
    }

    /// Publish a tick; returns any signals fired by this tick.
    async fn ingest_tick(
        &self,
        ctx: &Context<'_>,
        input: TickInput,
    ) -> Result<Vec<SignalObject>, Error> {
        let engine = ctx.data::<AppEngine>()?;
        let tick = MarketTick {
            instrument: InstrumentId::new(input.instrument).map_err(to_err)?,
            last_price: input.last_price,
            volume: input.volume,
            exchange_time: Utc::now(),
        };
        let signals = engine.process(tick).await.map_err(to_err)?;
        Ok(signals.into_iter().map(SignalObject::from).collect())
    }
}

/// Subscriptions.
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Stream signals, optionally filtered by rule ids (empty = all).
    async fn signals(
        &self,
        ctx: &Context<'_>,
        rule_ids: Vec<String>,
    ) -> Result<impl Stream<Item = SignalObject>, Error> {
        let engine = ctx.data::<AppEngine>()?;
        let mut ids = Vec::with_capacity(rule_ids.len());
        for id in rule_ids {
            ids.push(parse_rule_id(&id)?);
        }
        let stream = engine.subscribe(ids);
        Ok(stream.map(SignalObject::from))
    }
}

/// Build the GraphQL schema with depth/complexity guards and the engine in
/// context.
#[must_use]
pub fn build_schema(engine: AppEngine) -> ScanSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot)
        .limit_depth(12)
        .limit_complexity(512)
        .data(engine)
        .finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use scanengine_core::ScanConfig;

    fn schema() -> ScanSchema {
        let store = Arc::new(InMemoryRuleStore::new());
        let bus = Arc::new(BroadcastSignalBus::new(1024));
        let engine = Arc::new(ScanEngine::new(ScanConfig::default(), store, bus));
        build_schema(engine)
    }

    #[tokio::test]
    async fn add_rule_then_ingest_fires_signal() {
        let schema = schema();
        let add = r#"mutation {
            addRule(input: {name: "above-100", conditions: [{field: LAST_PRICE, comparator: GT, threshold: 100}]})
        }"#;
        let res = schema.execute(add).await;
        assert!(res.errors.is_empty(), "{:?}", res.errors);

        // First tick seeds below-threshold state.
        let seed = r#"mutation { ingestTick(input: {instrument: "NSE:TCS", lastPrice: 90, volume: 1}) { sequence } }"#;
        schema.execute(seed).await;

        let fire = r#"mutation {
            ingestTick(input: {instrument: "NSE:TCS", lastPrice: 150, volume: 2}) {
                ruleName instrument explanation
            }
        }"#;
        let res = schema.execute(fire).await;
        assert!(res.errors.is_empty(), "{:?}", res.errors);
        assert!(res.data.to_string().contains("above-100"));
    }

    #[tokio::test]
    async fn stats_query_works() {
        let schema = schema();
        let res = schema
            .execute("{ stats { ticksProcessed rules evaluationsPerTick } }")
            .await;
        assert!(res.errors.is_empty(), "{:?}", res.errors);
    }
}

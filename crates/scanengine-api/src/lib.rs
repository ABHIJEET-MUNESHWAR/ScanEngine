//! GraphQL API surface for ScanEngine: queries (rules, stats, state),
//! mutations (addRule, removeRule, ingestTick), and a signal subscription.

pub mod model;
pub mod schema;

pub use schema::{build_schema, AppEngine, MutationRoot, QueryRoot, ScanSchema, SubscriptionRoot};

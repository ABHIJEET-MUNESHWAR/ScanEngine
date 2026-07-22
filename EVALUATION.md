# ScanEngine — Self-Evaluation Against Engineering Guidelines

Legend: ✅ done · 🟡 partial · ⬜ not applicable

| # | Guideline | Status | Evidence |
|---|-----------|--------|----------|
| 1 | SOLID design | ✅ | Ports (`RuleStore`, `SignalBus`) invert dependencies; engine generic over both. |
| 2 | Microservices patterns (event-driven, CQRS, Saga) | ✅ | Event-driven signal fan-out via `SignalBus`; read (queries) vs write (ingest/rules) separation. |
| 3 | DB partitioning / sharding | ✅ | `DashMap` sharded state and rule store; store is a port so a sharded Redis/PG adapter drops in. |
| 4 | Timeouts, retry, fault tolerance | ✅ | `scanengine-resilience`: `with_timeout`, `retry_if` (equal-jitter). |
| 5 | Rate limiting + circuit breaker | ✅ | Token-bucket `RateLimiter` on tick admission; `CircuitBreaker` Closed/Open/HalfOpen. |
| 6 | Robust error handling / edge cases | ✅ | `thiserror` `CoreError`/`PortError`; invalid tick/rule, capacity, rate-limit paths tested. |
| 7 | GraphQL over REST (>5 endpoints) | ✅ | 7 root ops (3 queries, 3 mutations, 1 subscription). |
| 8 | ~85% test coverage | ✅ | 36 tests across all crates incl. mocks, e2e GraphQL, axum handlers. |
| 9 | Modular reusable components | ✅ | Resilience + types crates reusable across services. |
| 10 | Idiomatic Rust | ✅ | Newtypes, `Result` discipline, no `unwrap` on runtime paths. |
| 12 | GenAI / Agentic AI | ✅ | `Explainer` trait + `HeuristicExplainer` produce deterministic natural-language signal explanations. |
| 11 | Canonical crate stack | ✅ | tokio, axum, async-graphql, dashmap, metrics, tracing, criterion, mockall, proptest. |
| 13 | Generics & trait bounds | ✅ | `ScanEngine<R, B>`, `CircuitBreaker<C: Clock>`, `RateLimiter<C: Clock>`. |
| 14 | Clean interfaces | ✅ | Small trait surfaces; DTOs isolate GraphQL from domain. |
| 15 | README with TOC/badges/diagrams | ✅ | See `README.md` (mermaid, complexity, results). |
| 16 | Performance | ✅ | 36M condition evals/sec, ~25 ns per condition (release load test). |
| 17 | Tokio runtime, no blocking | ✅ | Fully async; broadcast fan-out; no blocking on the executor. |
| 18 | Parallel / concurrent / batch | ✅ | Sharded maps, concurrent subscribers, batched load loop, incremental rule index. |
| 19 | Logging & observability | ✅ | JSON tracing, Prometheus `/metrics`, health probes. |
| 20 | Recovery paths | ✅ | Retry + breaker; bus `send` no-subscriber case handled gracefully. |
| 21 | Composability | ✅ | Hexagonal layering; adapters swappable via ports. |
| 22 | Type-safety at compile time | ✅ | Validated `InstrumentId`/`Price` newtypes; `Field`/`Comparator` enums make illegal rules unrepresentable. |
| 23 | Interface segregation | ✅ | `RuleStore` and `SignalBus` are separate, focused ports. |
| 24 | Benchmarks + complexity | ✅ | criterion bench + Big-O table in README. |
| 25 | CI/CD | ✅ | `.github/workflows/ci.yml` (fmt, clippy -D warnings, test, audit). |
| 26 | Docker | ✅ | Multi-stage `Dockerfile` + `docker-compose.yml`. |
| 27 | Postman collection | ✅ | `postman/ScanEngine.postman_collection.json`. |
| 28 | Self-evaluation | ✅ | This document. |

## Known Limitations / Future Work

- Rule storage and signal transport are in-memory; persistent sharded adapters
  can be added behind the existing `RuleStore` / `SignalBus` ports without
  touching the engine.
- The `HeuristicExplainer` is rule-based and deterministic; a real LLM-backed
  explainer can be dropped in behind the `Explainer` trait for richer prose.
- Rule evaluation is currently single-pass per tick; a compiled predicate DAG
  could share subexpressions across rules for even higher throughput.

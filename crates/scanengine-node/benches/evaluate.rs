use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion};
use scanengine_core::{ScanConfig, ScanEngine};
use scanengine_infra::{BroadcastSignalBus, InMemoryRuleStore, TickGenerator};
use scanengine_types::{Comparator, Condition, Field, Scope};
use tokio::runtime::Runtime;

fn evaluate_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime");

    // Engine with 1000 Any-scoped rules (2000 conditions evaluated per tick).
    let engine = rt.block_on(async {
        let store = Arc::new(InMemoryRuleStore::new());
        let bus = Arc::new(BroadcastSignalBus::new(1024));
        let engine = Arc::new(ScanEngine::new(ScanConfig::default(), store, bus));
        for i in 0..1000 {
            engine
                .add_rule(
                    format!("r{i}"),
                    Scope::Any,
                    vec![
                        Condition {
                            field: Field::LastPrice,
                            comparator: Comparator::Gt,
                            threshold: 50 + (i as i64 % 400),
                        },
                        Condition {
                            field: Field::PctChangeBps,
                            comparator: Comparator::Gte,
                            threshold: -10_000,
                        },
                    ],
                )
                .await
                .unwrap();
        }
        engine
    });

    let mut generator = TickGenerator::new(1000, 7);

    let mut group = c.benchmark_group("evaluate");
    group.throughput(criterion::Throughput::Elements(1));
    group.bench_function("process_tick_1000_rules", |b| {
        b.iter_batched(
            || generator.next_tick(),
            |tick| {
                rt.block_on(async {
                    let _ = engine.process(tick).await;
                });
            },
            criterion::BatchSize::SmallInput,
        );
    });
    group.finish();
}

criterion_group!(benches, evaluate_benchmark);
criterion_main!(benches);

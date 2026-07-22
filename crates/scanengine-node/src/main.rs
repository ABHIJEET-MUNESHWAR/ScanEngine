mod config;
mod startup;
mod telemetry;

use std::sync::Arc;
use std::time::Instant;

use anyhow::Context as _;
use clap::Parser;
use config::{Cli, Command, LoadArgs, ServeArgs};
use hdrhistogram::Histogram;
use scanengine_api::build_schema;
use scanengine_core::{ScanConfig, ScanEngine};
use scanengine_infra::{BroadcastSignalBus, InMemoryRuleStore, TickGenerator};
use scanengine_types::{Comparator, Condition, Field, Scope};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    telemetry::init_tracing();

    let cli = Cli::parse();
    match cli
        .command
        .unwrap_or(Command::Serve(ServeArgs::parse_from(["serve"])))
    {
        Command::Serve(args) => serve(args).await,
        Command::Load(args) => load(args).await,
    }
}

async fn serve(args: ServeArgs) -> anyhow::Result<()> {
    let metrics = telemetry::install_metrics()?;

    let cfg = ScanConfig {
        max_instruments: args.max_instruments,
        ..ScanConfig::default()
    };
    let store = Arc::new(InMemoryRuleStore::new());
    let bus = Arc::new(BroadcastSignalBus::new(4_096));
    let engine = Arc::new(ScanEngine::new(cfg, store, bus));
    let schema = build_schema(engine);
    let app = startup::build_app(schema, metrics);

    let listener = tokio::net::TcpListener::bind(args.addr)
        .await
        .with_context(|| format!("bind {}", args.addr))?;
    tracing::info!(addr = %args.addr, "scanengine listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(startup::shutdown_signal())
        .await
        .context("server error")?;
    Ok(())
}

async fn load(args: LoadArgs) -> anyhow::Result<()> {
    let store = Arc::new(InMemoryRuleStore::new());
    let bus = Arc::new(BroadcastSignalBus::new(8_192));
    let engine = Arc::new(ScanEngine::new(ScanConfig::default(), store, bus));

    // Register Any-scoped rules so every tick evaluates the full rule base —
    // exercising the "thousands of conditions per tick" scenario. Thresholds
    // are spread across a wide percentage band so signals stay realistic.
    for i in 0..args.rules {
        let bps_threshold = 200 + (i as i64 % 2000) * 5; // +2% .. +52%
        engine
            .add_rule(
                format!("rule-{i}"),
                Scope::Any,
                vec![
                    Condition {
                        field: Field::PctChangeBps,
                        comparator: Comparator::CrossAbove,
                        threshold: bps_threshold,
                    },
                    Condition {
                        field: Field::LastPrice,
                        comparator: Comparator::Gt,
                        threshold: 0,
                    },
                ],
            )
            .await
            .context("register rule")?;
    }

    let mut generator = TickGenerator::new(args.instruments, 0x5CA1);
    let mut hist = Histogram::<u64>::new(3).context("create histogram")?;

    let start = Instant::now();
    for _ in 0..args.ticks {
        let tick = generator.next_tick();
        let t0 = Instant::now();
        let _ = engine.process(tick).await;
        let _ = hist.record(t0.elapsed().as_nanos() as u64);
    }
    let wall = start.elapsed();

    let stats = engine.stats();
    let throughput = args.ticks as f64 / wall.as_secs_f64();
    let eval_rate = stats.evaluations as f64 / wall.as_secs_f64();

    println!("ScanEngine load test");
    println!("  ticks processed     : {}", stats.ticks_processed);
    println!("  instruments         : {}", args.instruments);
    println!("  rules               : {}", args.rules);
    println!("  wall time           : {:.3}s", wall.as_secs_f64());
    println!("  tick throughput     : {throughput:.0} ticks/sec");
    println!("  condition eval rate : {eval_rate:.0} evals/sec");
    println!(
        "  evaluations/tick    : {:.1}",
        stats.evaluations_per_tick()
    );
    println!("  signals emitted     : {}", stats.signals_emitted);
    println!(
        "  eval p50 latency    : {} ns",
        hist.value_at_quantile(0.50)
    );
    println!(
        "  eval p99 latency    : {} ns",
        hist.value_at_quantile(0.99)
    );
    println!(
        "  eval p99.9 latency  : {} ns",
        hist.value_at_quantile(0.999)
    );
    Ok(())
}

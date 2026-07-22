use std::net::SocketAddr;

use clap::{Parser, Subcommand};

/// ScanEngine — a deterministic complex-event-processing scanner engine.
#[derive(Debug, Default, Parser)]
#[command(name = "scanengine-node", version, about)]
pub struct Cli {
    /// Subcommand to run (defaults to `serve`).
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Top-level commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the GraphQL + WebSocket server.
    Serve(ServeArgs),
    /// Run an in-process load test and report evaluation throughput.
    Load(LoadArgs),
}

/// Arguments for the `serve` command.
#[derive(Debug, Parser)]
pub struct ServeArgs {
    /// Address to bind the HTTP server to.
    #[arg(long, env = "SCANENGINE_ADDR", default_value = "0.0.0.0:8081")]
    pub addr: SocketAddr,

    /// Maximum number of distinct instruments.
    #[arg(long, env = "SCANENGINE_MAX_INSTRUMENTS", default_value_t = 100_000)]
    pub max_instruments: usize,
}

/// Arguments for the `load` command.
#[derive(Debug, Parser)]
pub struct LoadArgs {
    /// Number of synthetic instruments.
    #[arg(long, default_value_t = 2_000)]
    pub instruments: usize,

    /// Number of rules to register.
    #[arg(long, default_value_t = 1_000)]
    pub rules: usize,

    /// Total ticks to publish.
    #[arg(long, default_value_t = 1_000_000)]
    pub ticks: u64,
}

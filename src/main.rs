//! Solen L2 Rollup Node
//!
//! Sequences L2 transactions, publishes batches to L1, and bridges assets.
//!
//! Architecture:
//!   - Sequencer: orders incoming L2 transactions
//!   - Executor: runs transactions against L2 state (WASM VM)
//!   - Publisher: submits batch commitments to L1
//!   - Relayer: monitors L1 for deposits and bridges them to L2
//!   - RPC: JSON-RPC for L2 queries and transaction submission

mod executor;
mod publisher;
mod relayer;
mod rpc;

use std::net::SocketAddr;
use std::sync::Arc;

use clap::Parser;
use solen_rollup_kit::sequencer::{Sequencer, SequencerConfig};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "solen-rollup", version = "0.1.0")]
struct Cli {
    /// Rollup domain ID.
    #[arg(long, default_value = "1")]
    rollup_id: u64,

    /// L1 RPC endpoint.
    #[arg(long, default_value = "http://127.0.0.1:19944")]
    l1_rpc: String,

    /// L2 RPC listen port.
    #[arg(long, default_value = "3000")]
    port: u16,

    /// L2 data directory.
    #[arg(long, default_value = "data/rollup")]
    data_dir: String,

    /// Sequencer seed (32-byte hex).
    #[arg(long)]
    sequencer_seed: Option<String>,

    /// Batch interval in seconds.
    #[arg(long, default_value = "10")]
    batch_interval: u64,

    /// Max transactions per batch.
    #[arg(long, default_value = "100")]
    max_batch_size: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("=== Solen Rollup Node v0.1.0 ===");
    info!(
        rollup_id = cli.rollup_id,
        l1_rpc = %cli.l1_rpc,
        port = cli.port,
        batch_interval = cli.batch_interval,
    );

    // Initialize sequencer.
    let sequencer = Arc::new(Sequencer::new(SequencerConfig {
        rollup_id: cli.rollup_id,
        max_pending: 10_000,
        max_batch_size: cli.max_batch_size,
        batch_interval_ms: cli.batch_interval * 1000,
    }));

    // Initialize L2 state store.
    let l2_store = Arc::new(tokio::sync::RwLock::new(
        solen_storage::MemoryStore::new(),
    ));

    // Start batch publisher.
    let publisher_handle = {
        let seq = sequencer.clone();
        let l1_rpc = cli.l1_rpc.clone();
        let rollup_id = cli.rollup_id;
        let interval = cli.batch_interval;
        tokio::spawn(async move {
            publisher::run_publisher(seq, &l1_rpc, rollup_id, interval).await;
        })
    };

    // Start L1 deposit relayer.
    let relayer_handle = {
        let l1_rpc = cli.l1_rpc.clone();
        let rollup_id = cli.rollup_id;
        let store = l2_store.clone();
        tokio::spawn(async move {
            relayer::run_relayer(&l1_rpc, rollup_id, store).await;
        })
    };

    // Start L2 RPC server.
    let rpc_addr: SocketAddr = format!("0.0.0.0:{}", cli.port).parse()?;
    let rpc_handle = tokio::spawn(async move {
        if let Err(e) = rpc::start_rpc(rpc_addr, sequencer, l2_store).await {
            tracing::error!(error = %e, "L2 RPC failed");
        }
    });

    info!("Rollup node running. Press Ctrl+C to stop.");

    tokio::signal::ctrl_c().await?;
    info!("Shutting down");

    Ok(())
}

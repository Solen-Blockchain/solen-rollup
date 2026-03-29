//! L2 JSON-RPC server for the rollup node.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::response::Json;
use axum::routing::{get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use solen_rollup_kit::sequencer::{L2Transaction, Sequencer};
use solen_storage::MemoryStore;
use tracing::info;

#[derive(Clone)]
struct AppState {
    sequencer: Arc<Sequencer>,
    store: Arc<tokio::sync::RwLock<MemoryStore>>,
}

#[derive(Deserialize)]
struct SubmitTx {
    sender: String,
    nonce: u64,
    data: String,
    gas_limit: u64,
}

#[derive(Serialize)]
struct SubmitResult {
    accepted: bool,
    pending_count: usize,
}

#[derive(Serialize)]
struct RollupStatus {
    rollup_id: u64,
    pending_txs: usize,
    state_entries: usize,
}

pub async fn start_rpc(
    addr: SocketAddr,
    sequencer: Arc<Sequencer>,
    store: Arc<tokio::sync::RwLock<MemoryStore>>,
) -> anyhow::Result<()> {
    let state = AppState { sequencer, store };

    let app = Router::new()
        .route("/status", get(handle_status))
        .route("/submit", post(handle_submit))
        .route("/health", get(|| async { "ok" }))
        .with_state(state);

    info!(%addr, "L2 RPC server started");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handle_status(State(state): State<AppState>) -> Json<RollupStatus> {
    let store = state.store.read().await;
    Json(RollupStatus {
        rollup_id: state.sequencer.rollup_id(),
        pending_txs: state.sequencer.pending_count(),
        state_entries: store.len(),
    })
}

async fn handle_submit(
    State(state): State<AppState>,
    Json(tx): Json<SubmitTx>,
) -> Json<SubmitResult> {
    let sender = hex_decode_32(&tx.sender).unwrap_or([0u8; 32]);
    let data = hex::decode(&tx.data).unwrap_or_default();

    let l2_tx = L2Transaction {
        sender,
        nonce: tx.nonce,
        data,
        gas_limit: tx.gas_limit,
    };

    let accepted = state.sequencer.submit(l2_tx).is_ok();

    Json(SubmitResult {
        accepted,
        pending_count: state.sequencer.pending_count(),
    })
}

fn hex_decode_32(s: &str) -> Option<[u8; 32]> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes: Vec<u8> = (0..s.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect();
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

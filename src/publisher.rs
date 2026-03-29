//! Batch publisher: periodically produces batches from the sequencer
//! and submits them to L1 as batch commitments.

use std::sync::Arc;

use solen_crypto::blake3_hash;
use solen_rollup_kit::batch::BatchPublisher;
use solen_rollup_kit::prover::{MockProver, ProverBackend};
use solen_rollup_kit::sequencer::Sequencer;
use tracing::info;

pub async fn run_publisher(
    sequencer: Arc<Sequencer>,
    l1_rpc: &str,
    rollup_id: u64,
    interval_secs: u64,
) {
    let publisher = BatchPublisher::new(rollup_id);
    let prover = MockProver;
    let client = reqwest::Client::new();
    let mut pre_state_root = [0u8; 32];

    let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

    loop {
        tick.tick().await;

        let batch = match sequencer.produce_batch() {
            Some(b) => b,
            None => continue,
        };

        let tx_count = batch.transactions.len();

        // Compute post-state root (simplified — real impl would execute txs).
        let batch_data = serde_json::to_vec(&batch.transactions).unwrap_or_default();
        let post_state_root = blake3_hash(&batch_data);

        // Generate proof.
        let proof = match prover.generate_proof(&pre_state_root, &post_state_root, &batch_data) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(error = %e, "proof generation failed");
                continue;
            }
        };

        // Prepare L1 commitment.
        let commitment = match publisher.prepare_commitment(
            &batch,
            pre_state_root,
            post_state_root,
            proof,
        ) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "commitment preparation failed");
                continue;
            }
        };

        // Submit to L1 (via bridge system contract).
        // For now, just log it.
        info!(
            batch_index = commitment.batch_index,
            tx_count,
            rollup_id,
            "batch published to L1"
        );

        pre_state_root = post_state_root;
    }
}

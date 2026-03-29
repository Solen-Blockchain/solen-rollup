//! L2 transaction executor.
//!
//! Runs L2 transactions against the rollup's state using the WASM VM.
//! Produces state diffs for batch commitments.

use solen_storage::StateStore;
use solen_rollup_kit::sequencer::L2Transaction;

/// Execute a batch of L2 transactions against the state.
/// Returns the new state root.
pub fn execute_batch(
    store: &mut dyn StateStore,
    transactions: &[L2Transaction],
) -> [u8; 32] {
    for tx in transactions {
        // For now, store transaction data keyed by sender+nonce.
        let key = format!("tx/{}/{}", hex(&tx.sender), tx.nonce);
        let _ = store.put(key.as_bytes(), &tx.data);
    }

    store.commit_root();
    store.state_root()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

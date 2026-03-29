//! L1 deposit relayer: monitors L1 for deposit events and credits L2 accounts.

use std::sync::Arc;

use solen_storage::MemoryStore;
use tracing::{debug, info};

pub async fn run_relayer(
    l1_rpc: &str,
    rollup_id: u64,
    l2_store: Arc<tokio::sync::RwLock<MemoryStore>>,
) {
    let client = reqwest::Client::new();
    let mut last_checked_height = 0u64;
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

    info!(rollup_id, "L1 relayer started");

    loop {
        interval.tick().await;

        // Poll L1 for new blocks.
        let status = match get_l1_height(&client, l1_rpc).await {
            Ok(h) => h,
            Err(e) => {
                debug!(error = %e, "failed to poll L1");
                continue;
            }
        };

        if status <= last_checked_height {
            continue;
        }

        // Check new blocks for deposit events.
        // In a full implementation, we'd parse block events for bridge deposits
        // targeting this rollup_id and credit L2 accounts.
        debug!(
            from = last_checked_height + 1,
            to = status,
            "scanning L1 blocks for deposits"
        );

        last_checked_height = status;
    }
}

async fn get_l1_height(client: &reqwest::Client, rpc_url: &str) -> anyhow::Result<u64> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "solen_chainStatus",
        "params": [],
        "id": 1
    });

    let resp: serde_json::Value = client.post(rpc_url).json(&body).send().await?.json().await?;

    resp["result"]["height"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("failed to parse L1 height"))
}

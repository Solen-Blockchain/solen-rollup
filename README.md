# Solen Rollup Node

L2 rollup node for the Solen network. Sequences transactions, publishes batches to L1, and bridges assets.

## Architecture

```
L2 Users → [L2 RPC] → [Sequencer] → [Executor] → [Publisher] → L1
                                                        ↓
L1 → [Relayer] → L2 state (deposits)
```

## Running

```bash
cargo run --bin solen-rollup -- \
    --rollup-id 1 \
    --l1-rpc http://127.0.0.1:19944 \
    --port 3000
```

## L2 RPC Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/status` | GET | Rollup status (pending txs, state entries) |
| `/submit` | POST | Submit an L2 transaction |
| `/health` | GET | Health check |

### Submit a transaction

```bash
curl -X POST http://localhost:3000/submit \
  -H "Content-Type: application/json" \
  -d '{"sender": "<hex>", "nonce": 0, "data": "<hex>", "gas_limit": 100}'
```

## CLI Options

```
Options:
    --rollup-id <ID>           Rollup domain ID [default: 1]
    --l1-rpc <URL>             L1 RPC endpoint [default: http://127.0.0.1:19944]
    --port <PORT>              L2 RPC port [default: 3000]
    --data-dir <DIR>           Data directory [default: data/rollup]
    --batch-interval <SECS>    Batch interval [default: 10]
    --max-batch-size <N>       Max txs per batch [default: 100]
```

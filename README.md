<p align="center">
  <h1 align="center">Vigil Protection RPC</h1>
  <p align="center">
    A Solana RPC proxy that shields swap transactions from sandwich attacks.<br/>
    Detects DEX swaps across 8 protocols, evaluates risk in real-time using a learned pool/attacker map, decodes instruction-level slippage, and routes risky transactions through Jito's private relay.
  </p>
</p>

<p align="center">
  <strong>English</strong> &middot;
  <a href="README.ko.md">한국어</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Solana-9945FF?style=for-the-badge&logo=solana&logoColor=white" alt="Solana" />
  <img src="https://img.shields.io/badge/Axum-232323?style=for-the-badge&logo=rust&logoColor=white" alt="Axum" />
  <img src="https://img.shields.io/badge/Jito-FF6B35?style=for-the-badge" alt="Jito" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=for-the-badge" alt="MIT License" />
</p>

<p align="center">
  <a href="#how-it-works">How It Works</a> &middot;
  <a href="#quick-start">Quick Start</a> &middot;
  <a href="#configuration">Configuration</a> &middot;
  <a href="#supported-dexes">DEXes</a> &middot;
  <a href="#api">API</a> &middot;
  <a href="#architecture">Architecture</a>
</p>

---

Sandwich attacks are the most common MEV exploit on Solana. An attacker front-runs a victim's swap, moves the price, and back-runs to profit. **Vigil Protection RPC** sits between the user and the network, analyzing every `sendTransaction` in real-time and routing swap transactions through Jito's private relay to prevent mempool exposure.

> Detection primitive powered by [solana-sandwich-detector](https://github.com/SangHyeonKwon/solana-sandwich-detector). This repo is the **runtime protection layer** — detection stays upstream, protection lives here.

## How It Works

```
                        sendTransaction
                              |
                    +---------v----------+
                    |   Swap Detection   |
                    |  (8 DEX programs)  |
                    +---------+----------+
                              |
                      swap?   |   not swap
                  +-----------+-----------+
                  |                       |
        +---------v----------+   +--------v--------+
        |  Slippage Decode   |   |   Direct RPC    |
        | (Raydium, Orca,    |   | (Helius staked) |
        |  Jupiter, Pump.fun)|   +---------+-------+
        +---------+----------+             |
                  |                        |
        +---------v----------+             |
        |   Risk Assessment  |             |
        | pool_score x slip  |             |
        | + attacker lookup  |             |
        +---------+----------+             |
                  |                        |
          +-------v--------+               |
          | Jito Relay     |--fail--+      |
          | (private send) |        |      |
          +-------+--------+        |      |
                  |             +---v------v---+
                  |             | Direct RPC   |
                  v             | (fallback)   |
               response        +------+-------+
                                      |
                                      v
                                   response

  [Background: Slot Watcher]
  Polls mainnet blocks --> extract_swaps --> detect_sandwiches
  --> updates Pool Risk Map + Attacker Set (in-memory LRU)
```

### Decision matrix

| Condition | Route | Why |
|-----------|-------|-----|
| Not a swap | Direct RPC | No MEV risk |
| Swap, Jito enabled | Jito relay (fallback: Direct RPC) | Private mempool, no sandwich |
| Swap, Jito disabled | Direct RPC | User opted out of Jito |
| Swap, BLOCK + strict mode | JSON-RPC error | Hard reject high-risk TX |

## Quick Start

```bash
# Clone & build
git clone https://github.com/EarthIsMine/Vigil-RPC.git
cd Vigil-RPC
cargo build --release

# Configure (copy and edit .env)
cp .env.example .env
# Set SOLANA_RPC_URL and SOLANA_SEND_TX_URL to your Helius endpoints

# Run
RUST_LOG=info cargo run --release

# Verify
curl http://localhost:8899/health
```

Point your wallet or dApp at `http://localhost:8899` instead of the default Solana RPC. All JSON-RPC methods are proxied transparently; `sendTransaction` gets the protection layer.

## Supported DEXes

### Swap detection (8 protocols)

| DEX | Program ID | Pool index |
|-----|-----------|------------|
| Raydium V4 | `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` | accounts[1] |
| Raydium CLMM | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` | accounts[2] |
| Raydium CPMM | `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C` | accounts[3] |
| Orca Whirlpool | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | accounts[2] |
| Jupiter V6 | `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4` | router (no single pool) |
| Meteora DLMM | `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo` | accounts[0] |
| Pump.fun | `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P` | accounts[2] |
| Phoenix | `PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY` | accounts[0] |

### Slippage decode (4 protocols)

| DEX | What's decoded | Unbounded signal |
|-----|---------------|-----------------|
| Raydium V4 | `min_amount_out` / `max_amount_in` from instruction data | `min_out == 0` or `max_in == MAX` |
| Orca Whirlpool | `other_amount_threshold` from Anchor swap | threshold == 0 (ExactIn) |
| Jupiter V6 | `slippage_bps` from last 3 bytes | bps > 500 (5%) |
| Pump.fun | `max_sol_cost` (buy) / `min_sol_output` (sell) | cost == MAX or output == 0 |

## Configuration

All configuration is via environment variables. A `.env` file is auto-loaded on startup.

| Variable | Default | Description |
|----------|---------|-------------|
| `PROTECTION_RPC_PORT` | `8899` | Server listen port |
| `SOLANA_RPC_URL` | `https://api.mainnet-beta.solana.com` | Upstream RPC for read queries + slot watcher |
| `SOLANA_SEND_TX_URL` | same as `SOLANA_RPC_URL` | Staked RPC for Direct sendTransaction |
| `JITO_ENABLED` | `true` | Route swap TXs through Jito relay |
| `JITO_BLOCK_ENGINE_URL` | `https://mainnet.block-engine.jito.wtf` | Jito Block Engine endpoint |
| `BLOCK_MODE` | `permissive` | `permissive` = log only, `strict` = reject BLOCK-level TXs |
| `SLOT_POLL_INTERVAL_SECS` | `10` | Slot watcher polling frequency |
| `SLOT_WATCHER_LAG` | `32` | Slots behind head to poll (finalization buffer) |
| `POOL_LRU_CAPACITY` | `1024` | Pool risk map LRU cache size |
| `ATTACKER_LRU_CAPACITY` | `1024` | Attacker set LRU cache size |
| `POOL_TTL_SECS` | `86400` | TTL before pool/attacker entries expire |
| `RUST_LOG` | - | Log level (`info`, `debug`, `trace`) |

## API

### `GET /health`

Returns server status, current block mode, and all metrics counters.

```json
{
  "status": "ok",
  "block_mode": "Permissive",
  "metrics": {
    "risk_safe_total": 142,
    "risk_warn_total": 23,
    "risk_block_total": 5,
    "blocked_strict_total": 0,
    "forwarded_total": 1205,
    "jito_routed_total": 170,
    "slot_processed_total": 3400,
    "slot_failed_total": 12,
    "sandwich_detected_total": 8
  }
}
```

### `POST /`

Standard Solana JSON-RPC proxy. All methods pass through transparently. `sendTransaction` gets the protection pipeline.

```bash
# Regular RPC call (pass-through)
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# Protected sendTransaction
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"sendTransaction","params":["BASE64_TX"]}'
```

## Architecture

```
src/
  main.rs                  Entry point, AppState init, slot watcher spawn
  config.rs                Env-based configuration (Jito, risk, polling)
  state.rs                 Shared state (Arc-wrapped senders, maps, metrics)
  error.rs                 JSON-RPC compatible error types
  metrics.rs               Atomic counters, /health snapshot

  analyzer/
    swap_detector.rs       DEX_LAYOUTS table, detect_swap, extract_pool_address
    tx_parser.rs           Base64/base58 → VersionedTransaction decode

  risk/
    pool_map.rs            PoolRiskMap — LRU + TTL, score from sandwich history
    attacker_set.rs        AttackerSet — known sandwich attacker signers
    decision.rs            assess() → SAFE / WARN / BLOCK
    slippage.rs            Instruction-level slippage decode (4 DEX families)

  slot_watcher/
    poller.rs              Background tokio task, getBlock → detect_sandwiches

  transmission/
    rpc_forward.rs         Direct RPC via solana-client
    jito_sender.rs         Jito Block Engine relay (sendTransaction + sendBundle)

  server/
    routes.rs              Axum router (/health, /)
    handlers/rpc.rs        sendTransaction interception + risk routing
    rpc_types.rs           JSON-RPC request/response types
```

### Key dependency

[solana-sandwich-detector](https://github.com/SangHyeonKwon/solana-sandwich-detector) v1.0.1 provides:
- `dex::all_parsers()` — 8 DEX parsers
- `dex::extract_swaps()` — post-execution swap extraction (slot watcher)
- `detector::detect_sandwiches()` — same-block pattern matching
- `parser::parse_block()` — RPC block → internal BlockData

## Tests

```bash
cargo test                    # All 25 tests
cargo test risk_pipeline      # Risk assessment pipeline (8 tests)
cargo test http_integration   # HTTP handler tests (3 tests)
cargo test slippage           # Slippage decode tests (7 tests)
```

## License

MIT

<p align="center">
  <h1 align="center">Vigil Protection RPC</h1>
  <p align="center">
    스왑 트랜잭션을 샌드위치 공격으로부터 보호하는 Solana RPC 프록시.<br/>
    8개 DEX 프로토콜의 스왑을 감지하고, 학습된 풀/공격자 맵으로 실시간 위험도를 평가하며, instruction 수준의 슬리피지를 디코드하고, 위험한 트랜잭션을 Jito 프라이빗 릴레이로 라우팅합니다.
  </p>
</p>

<p align="center">
  <a href="README.md">English</a> &middot;
  <strong>한국어</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/Solana-9945FF?style=for-the-badge&logo=solana&logoColor=white" alt="Solana" />
  <img src="https://img.shields.io/badge/Axum-232323?style=for-the-badge&logo=rust&logoColor=white" alt="Axum" />
  <img src="https://img.shields.io/badge/Jito-FF6B35?style=for-the-badge" alt="Jito" />
  <img src="https://img.shields.io/badge/License-MIT-blue?style=for-the-badge" alt="MIT License" />
</p>

<p align="center">
  <a href="#동작-원리">동작 원리</a> &middot;
  <a href="#빠른-시작">빠른 시작</a> &middot;
  <a href="#설정">설정</a> &middot;
  <a href="#지원-dex">DEX</a> &middot;
  <a href="#api">API</a> &middot;
  <a href="#아키텍처">아키텍처</a>
</p>

---

샌드위치 공격은 Solana에서 가장 흔한 MEV 착취 방식입니다. 공격자가 피해자의 스왑을 선행매매(front-run)하여 가격을 움직인 후, 후행매매(back-run)로 이익을 취합니다. **Vigil Protection RPC**는 사용자와 네트워크 사이에 위치하여 모든 `sendTransaction`을 실시간으로 분석하고, 스왑 트랜잭션을 Jito 프라이빗 릴레이를 통해 전송하여 멤풀 노출을 방지합니다.

> 탐지 엔진: [solana-sandwich-detector](https://github.com/SangHyeonKwon/solana-sandwich-detector). 이 레포는 **런타임 보호 레이어**입니다 — 탐지는 upstream에, 보호는 여기에.

## 동작 원리

```
                        sendTransaction
                              |
                    +---------v----------+
                    |    스왑 감지        |
                    |  (8개 DEX 프로그램) |
                    +---------+----------+
                              |
                     스왑?    |   스왑 아님
                  +-----------+-----------+
                  |                       |
        +---------v----------+   +--------v--------+
        |  슬리피지 디코드    |   |   Direct RPC    |
        | (Raydium, Orca,    |   | (Helius staked) |
        |  Jupiter, Pump.fun)|   +---------+-------+
        +---------+----------+             |
                  |                        |
        +---------v----------+             |
        |    위험도 평가      |             |
        | pool_score x slip  |             |
        | + 공격자 조회       |             |
        +---------+----------+             |
                  |                        |
          +-------v--------+               |
          | Jito 릴레이    |--실패--+      |
          | (프라이빗 전송) |        |      |
          +-------+--------+        |      |
                  |             +---v------v---+
                  |             | Direct RPC   |
                  v             | (폴백)       |
               응답             +------+-------+
                                      |
                                      v
                                    응답

  [백그라운드: 슬롯 워처]
  메인넷 블록 폴링 --> extract_swaps --> detect_sandwiches
  --> Pool Risk Map + Attacker Set 갱신 (인메모리 LRU)
```

### 결정 매트릭스

| 조건 | 라우팅 | 이유 |
|------|--------|------|
| 스왑 아님 | Direct RPC | MEV 위험 없음 |
| 스왑, Jito 활성화 | Jito 릴레이 (실패 시 Direct RPC 폴백) | 프라이빗 멤풀, 샌드위치 불가 |
| 스왑, Jito 비활성화 | Direct RPC | 사용자가 Jito 비활성화 |
| 스왑, BLOCK + strict 모드 | JSON-RPC 에러 | 고위험 TX 거부 |

## 빠른 시작

```bash
# 클론 & 빌드
git clone https://github.com/EarthIsMine/Vigil-RPC.git
cd Vigil-RPC
cargo build --release

# 설정 (.env 복사 후 편집)
cp .env.example .env
# SOLANA_RPC_URL과 SOLANA_SEND_TX_URL에 Helius 엔드포인트 설정

# 실행
RUST_LOG=info cargo run --release

# 확인
curl http://localhost:8899/health
```

지갑이나 dApp의 RPC를 `http://localhost:8899`로 변경하면 됩니다. 모든 JSON-RPC 메서드는 투명하게 프록시되고, `sendTransaction`만 보호 파이프라인을 거칩니다.

## 지원 DEX

### 스왑 감지 (8개 프로토콜)

| DEX | Program ID | Pool 인덱스 |
|-----|-----------|------------|
| Raydium V4 | `675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8` | accounts[1] |
| Raydium CLMM | `CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK` | accounts[2] |
| Raydium CPMM | `CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C` | accounts[3] |
| Orca Whirlpool | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | accounts[2] |
| Jupiter V6 | `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4` | 라우터 (단일 풀 없음) |
| Meteora DLMM | `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo` | accounts[0] |
| Pump.fun | `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P` | accounts[2] |
| Phoenix | `PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY` | accounts[0] |

### 슬리피지 디코드 (4개 프로토콜)

| DEX | 디코드 대상 | 무제한 슬리피지 신호 |
|-----|------------|---------------------|
| Raydium V4 | instruction data의 `min_amount_out` / `max_amount_in` | `min_out == 0` 또는 `max_in == MAX` |
| Orca Whirlpool | Anchor swap의 `other_amount_threshold` | threshold == 0 (ExactIn) |
| Jupiter V6 | 마지막 3바이트의 `slippage_bps` | bps > 500 (5%) |
| Pump.fun | `max_sol_cost` (매수) / `min_sol_output` (매도) | cost == MAX 또는 output == 0 |

## 설정

모든 설정은 환경변수로 관리됩니다. `.env` 파일은 서버 시작 시 자동으로 로드됩니다.

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `PROTECTION_RPC_PORT` | `8899` | 서버 리스닝 포트 |
| `SOLANA_RPC_URL` | `https://api.mainnet-beta.solana.com` | 읽기 쿼리 + 슬롯 워처용 업스트림 RPC |
| `SOLANA_SEND_TX_URL` | `SOLANA_RPC_URL`과 동일 | sendTransaction용 Staked RPC |
| `JITO_ENABLED` | `true` | 스왑 TX를 Jito 릴레이로 라우팅 |
| `JITO_BLOCK_ENGINE_URL` | `https://mainnet.block-engine.jito.wtf` | Jito Block Engine 엔드포인트 |
| `BLOCK_MODE` | `permissive` | `permissive` = 로그만, `strict` = BLOCK 수준 TX 거부 |
| `SLOT_POLL_INTERVAL_SECS` | `10` | 슬롯 워처 폴링 주기(초) |
| `SLOT_WATCHER_LAG` | `32` | 폴링 시 헤드 대비 지연 슬롯 수 |
| `POOL_LRU_CAPACITY` | `1024` | Pool Risk Map LRU 캐시 크기 |
| `ATTACKER_LRU_CAPACITY` | `1024` | Attacker Set LRU 캐시 크기 |
| `POOL_TTL_SECS` | `86400` | 풀/공격자 항목 TTL(초) |
| `RUST_LOG` | - | 로그 레벨 (`info`, `debug`, `trace`) |

## API

### `GET /health`

서버 상태, 현재 블록 모드, 전체 메트릭 카운터를 반환합니다.

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

표준 Solana JSON-RPC 프록시. 모든 메서드는 투명하게 패스스루됩니다. `sendTransaction`만 보호 파이프라인을 거칩니다.

```bash
# 일반 RPC 호출 (패스스루)
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'

# 보호된 sendTransaction
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"sendTransaction","params":["BASE64_TX"]}'
```

## 아키텍처

```
src/
  main.rs                  진입점, AppState 초기화, 슬롯 워처 spawn
  config.rs                환경변수 기반 설정 (Jito, risk, polling)
  state.rs                 공유 상태 (Arc 래핑된 sender, map, metrics)
  error.rs                 JSON-RPC 호환 에러 타입
  metrics.rs               Atomic 카운터, /health 스냅샷

  analyzer/
    swap_detector.rs       DEX_LAYOUTS 테이블, detect_swap, extract_pool_address
    tx_parser.rs           Base64/base58 → VersionedTransaction 디코드

  risk/
    pool_map.rs            PoolRiskMap — LRU + TTL, 샌드위치 이력 기반 점수
    attacker_set.rs        AttackerSet — 알려진 샌드위치 공격자 signer
    decision.rs            assess() → SAFE / WARN / BLOCK
    slippage.rs            Instruction 수준 슬리피지 디코드 (4개 DEX 패밀리)

  slot_watcher/
    poller.rs              백그라운드 tokio 태스크, getBlock → detect_sandwiches

  transmission/
    rpc_forward.rs         solana-client 기반 Direct RPC
    jito_sender.rs         Jito Block Engine 릴레이 (sendTransaction + sendBundle)

  server/
    routes.rs              Axum 라우터 (/health, /)
    handlers/rpc.rs        sendTransaction 인터셉트 + 위험도 기반 라우팅
    rpc_types.rs           JSON-RPC 요청/응답 타입
```

### 핵심 의존성

[solana-sandwich-detector](https://github.com/SangHyeonKwon/solana-sandwich-detector) v1.0.1:
- `dex::all_parsers()` — 8개 DEX 파서
- `dex::extract_swaps()` — 실행 후 스왑 추출 (슬롯 워처 전용)
- `detector::detect_sandwiches()` — 동일 블록 패턴 매칭
- `parser::parse_block()` — RPC 블록 → 내부 BlockData

## 테스트

```bash
cargo test                    # 전체 25개 테스트
cargo test risk_pipeline      # 위험도 평가 파이프라인 (8개)
cargo test http_integration   # HTTP 핸들러 (3개)
cargo test slippage           # 슬리피지 디코드 (7개)
```

## 라이선스

MIT

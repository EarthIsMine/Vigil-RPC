# Vigil Protection RPC

Solana MEV 보호 RPC 프록시. 사용자 트랜잭션을 받아 샌드위치 공격으로부터 보호한다.

## 빌드

```bash
cargo build
```

## 실행

```bash
# 기본 설정 (포트 8899, mainnet RPC)
cargo run

# 환경변수 설정
SOLANA_RPC_URL=https://your-rpc-url.com RUST_LOG=info cargo run
```

## 환경변수

| 변수 | 기본값 | 설명 |
|------|--------|------|
| `PROTECTION_RPC_PORT` | `8899` | 서버 포트 |
| `SOLANA_RPC_URL` | `https://api.mainnet-beta.solana.com` | 업스트림 Solana RPC URL |
| `RUST_LOG` | - | 로그 레벨 (`info`, `debug`, `trace`) |

## 테스트

```bash
# Health check
curl http://localhost:8899/health

# JSON-RPC pass-through
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"getHealth"}'
```

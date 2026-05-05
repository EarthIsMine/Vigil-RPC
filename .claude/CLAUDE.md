# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 프로젝트 개요

Solana MEV 보호 RPC 프록시. 사용자 TX를 받아 sandwich 공격 위험도를 평가하고, 위험하면 Jito 번들로 보호, 안전하면 Direct RPC로 포워딩한다. Vigil 대시보드(FE/BE)와 **완전히 독립적**인 Rust 서비스.

## 빌드/테스트/실행

```bash
cargo build              # 빌드
cargo test               # 전체 테스트 (25개)
cargo test risk_pipeline # 특정 모듈 테스트
cargo test ttl_expiry    # 특정 테스트 하나
RUST_LOG=info cargo run  # 서버 실행 (.env 자동 로드)
```

## 아키텍처 — 3-Layer Adaptive Filter

```
[sendTransaction 수신]
  ↓
① detect_swap() — DEX_LAYOUTS 테이블로 8개 DEX 프로그램 매칭
  swap 아니면 → Direct RPC 즉시 통과
  ↓
② decode_slippage() — instruction data에서 min_amount_out/slippage_bps 추출
  (Raydium V4, Orca Whirlpool, Jupiter V6, Pump.fun 지원)
  ↓
③ extract_pool_address() → pool_map.score() + attacker_set 조회
  ↓
④ assess() → SAFE / WARN / BLOCK 결정
  BLOCK + strict → JSON-RPC 에러 반환
  그 외 → Direct RPC 포워딩 (Tier 2에서 Jito 분기 추가 예정)

[Background: Slot Watcher]
  매 N초 getBlock 폴링 → parse_block → extract_swaps → detect_sandwiches
  → PoolRiskMap + AttackerSet 갱신 (인메모리 LRU + TTL)
```

## 핵심 모듈 관계

- **`analyzer/swap_detector.rs`**: `DEX_LAYOUTS` 상수 테이블이 8개 DEX의 program_id + pool account index를 정의. `detect_swap()`, `extract_pool_address()`, `signer_as_string()` 제공.
- **`risk/`**: `pool_map`(PoolRiskMap)과 `attacker_set`(AttackerSet)은 slot_watcher가 갱신하고 handler가 읽는다. `decision::assess()`가 최종 RiskLevel 결정. `slippage.rs`는 DEX별 instruction data를 디코드.
- **`slot_watcher/poller.rs`**: sandwich-detector 크레이트의 `parse_block()` + `extract_swaps()` + `detect_sandwiches()`를 호출. 슬롯 건너뛰기 방지(catch-up 루프), skipped slot 감지, 네트워크 에러 시 break.
- **`server/handlers/rpc.rs`**: `sendTransaction` 만 인터셉트, 나머지는 upstream pass-through. `evaluate_risk()`가 pool 식별 → score → slippage → assess 파이프라인 실행.
- **`state.rs`**: `AppState`에 `pool_map`, `attacker_set`, `metrics`가 `Arc`로 공유. Clone으로 핸들러/워처 간 전달.

## 외부 의존성: sandwich-detector 크레이트

`sandwich-detector` (git dep, v1.0.1)는 별도 레포 ([SangHyeonKwon/solana-sandwich-detector](https://github.com/SangHyeonKwon/solana-sandwich-detector)). 8 DEX 파서 제공. 주요 API:
- `dex::all_parsers()` → `Vec<Box<dyn DexParser>>`
- `dex::extract_swaps(tx: &TransactionData, parsers)` → `Vec<SwapEvent>` (post-execution 데이터 필요, handler에서 직접 사용 불가)
- `detector::detect_sandwiches(slot, &[SwapEvent])` → `Vec<SandwichAttack>`
- `parser::parse_block(slot, UiConfirmedBlock)` → `BlockData`

**주의**: `extract_swaps`는 `TransactionData`(잔고 변화 포함)가 필요하므로 slot_watcher에서만 사용. 핸들러 시점의 단일 TX(`VersionedTransaction`)에서는 program_id 매칭 + instruction account index로만 판단.

## 구현 원칙

- **Tier 순서**: Tier 1(Proxy) ✅ → Tier 2(Jito) 🔜 → Tier 3(Leader-aware)
- **BE 없이 동작**: Tier 2까지 Backend API 호출 없이 완전 독립
- **sendTransaction 100% 호환**: Solana JSON-RPC spec 준수
- **BLOCK_MODE=permissive 기본**: 잘못된 차단은 사용자 신뢰 손상. strict는 명시 토글 후에만
- Jito Block Engine은 공개 endpoint (`mainnet.block-engine.jito.wtf`), 별도 keypair 발급 불필요. 번들당 tip(SOL)만 지불

## 참조 문서

구현 전에 `.context/attachments/` 문서를 순서대로 읽을 것:
1. `00-project-overview.md` — 전체 아키텍처
2. `01-protection-rpc-full-spec.md` — 상세 구현 명세
3. `02-implementation-tiers.md` — 단계별 로드맵
4. `03-backend-internal-api-contract.md` — Backend API 계약 (Tier 3)
5. `04-known-risks-and-experiments.md` — 리스크 항목

## 주의사항

- `solana-sdk v2` + sandwich-detector 조합에서 Cargo 버전 충돌 가능. `cargo tree -d`로 확인.
- `getBlock` RPC 비용: Helius 호출 한도 주의. `SLOT_POLL_INTERVAL_SECS=10` 기본.
- `RwLock` poisoning: pool_map/attacker_set에서 `Ok` guard 패턴 사용 (panic 대신 graceful return).

# Cycle for Trae Work — Roadmap

Status after 1.0.0. Everything here is direction, not commitment; nothing ships without passing the same certification gates as 1.0.

## 1.1 — Scale throughput

The 500k-file benchmark certified correctness and incremental refresh (34.9 s) with a cold-index wall clock dominated by SQLite partition persistence. Planned work: batched partition writes and connection tuning to bring the cold index under the benchmark's 30-minute gate on SATA-class storage. No format changes; existing indexes stay valid.

## 1.2 — Certification breadth

- Linux x64 desktop certification run mirrored on the CI matrix, with the same scripted suite used on Windows.
- Repeat certification on the 500k corpus with NVMe storage to publish the throughput characterization.

## 1.3 — Language coverage

Additional tree-sitter grammars for the inventory and graph (candidates: Kotlin, Swift, Scala, PHP). Each grammar lands with reconcile tests and incremental-refresh coverage; unsupported languages keep inventory-only behavior.

## 1.4 — Role endpoint hardening

- Optional response pinning (expected model fingerprint) for role endpoints.
- Per-role request budgets with explicit user-visible exhaustion, complementing the token usage ledger.

## Later

- Plugin marketplace packaging if Trae Work introduces a first-class distribution channel; the allowlist pipeline already produces the required artifacts.
- Optional signed checkpoints export for teams that want an offline notarized chain head.
- Multi-worktree parallel execution inside one workflow, under the existing admission controller.

## Non-goals

Cloud execution, hosted dashboards, vector databases, embeddings, telemetry, or any approval path that bypasses the isolated arbiter.

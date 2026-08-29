# Cycle for Trae Work — Roadmap

Status after 1.0.0. Everything here is direction, not commitment; nothing ships without passing the same certification gates as 1.0.

## 1.0.1 candidate — Scale throughput

The 500k-file benchmark now passes the unchanged 30-minute gate on the SATA-class Windows certification host. Large cold indexes transactionally defer three secondary indexes, insert graph/manifest/FTS rows in bounded multi-row batches, rebuild the indexes before commit, and use a 256 MiB SQLite cache ceiling. No schema or stored-format change is required; existing indexes remain valid. The optimization receipt on `c9ff3ef` records 23m 38.4s total, 0 parse errors and a 46.1s incremental refresh. Final release sealing must repeat the full benchmark on the final SHA.

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

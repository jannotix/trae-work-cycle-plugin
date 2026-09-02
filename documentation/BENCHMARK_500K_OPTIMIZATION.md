# 500k Index Optimization Ledger

Status: **full 500k gate passed on the current cache-ceiling commit; final release SHA rerun required**

The release gate remains unchanged: more than 500,000 physical and inventoried
files, 500,100 parsed source files, zero parse errors, correct query and
incremental reconciliation, peak memory at or below 80% of the host, and both
cold indexing and total wall time within 30 minutes.

## Search contract

- Baseline: the tracked Windows report (`certification-500k.json`) plus a fresh
  50,100-source-file replay on the current toolchain.
- Correctness gate: identical node/edge counts for the same corpus, zero parse
  errors, oracle route found, and modify/rename/delete reconciliation all true.
- Primary metric: persistence milliseconds; secondary metrics: complete index,
  total wall time, incremental refresh and peak memory.
- Budget: at most three variants. Each variant changes one performance
  hypothesis and is compared with the prior accepted winner.
- Rollback: each optimization is confined to the graph-store/index path and can
  be reverted independently without a schema migration.

## Controlled 50k replay

Command shape:

```text
cargo run --release -p workflow-code-intel --example codebase_500k --locked -- --source-files 50100 --ignored-files 2000 --output <receipt.json>
```

The reduced corpus intentionally exits 1 because it is below the mandatory
500k size. The report is still the controlled optimization measurement; only a
full corpus can pass release certification.

| Variant | Hypothesis | Persistence | Index | Total | Incremental | Peak memory | Correct |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| Baseline | Existing prepared single-row inserts and live secondary indexes | 66,224 ms | 68,834 ms | 125,926 ms | 8,912 ms | 231.4 MiB | Yes |
| V1 | Drop three graph secondary indexes inside the cold-index transaction and rebuild them before commit | 45,842 ms | 47,983 ms | 100,721 ms | 7,250 ms | 220.1 MiB | Yes |
| V2 | V1 plus 128-row inserts for nodes, edges, manifest and FTS paths | 41,076 ms | 43,075 ms | 101,569 ms | 7,126 ms | 217.4 MiB | Yes |
| V3 | V2 plus a 256 MiB SQLite cache ceiling for the graph writer | **32,031 ms** | **34,078 ms** | **93,762 ms** | **4,543 ms** | 407.3 MiB | Yes |

## Post-promotion host-headroom revalidation

The final Windows host temporarily had less free memory than the earlier full
run's 1,025.7 MiB process peak. This is a separate, bounded cache-ceiling
decision: all variants use the same 50,100-source-file corpus and all preserve
the parsing, query, and modify/rename/delete oracles.

| Variant | Hypothesis | Persistence | Index | Total | Incremental | Peak memory | Correct | Decision |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | --- |
| 256 MiB baseline | Existing V3 cache ceiling | 21,606 ms | 22,938 ms | 60,543 ms | 2,681 ms | 419.4 MiB | Yes | Baseline |
| 128 MiB | Maximize memory reduction | 42,446 ms | 44,678 ms | 98,465 ms | 2,681 ms | 289.6 MiB | Yes | Rejected: 62.6% slower total |
| V4: 192 MiB | Preserve host headroom with bounded cache loss | 22,953 ms | 24,356 ms | 63,264 ms | 2,681 ms | 353.8 MiB | Yes | **Promoted** |

## Selected implementation

V4 is the best measured safe variant for the release host. Its 192 MiB cache
ceiling saves 65.6 MiB on the controlled replay versus the 256 MiB baseline
while adding only 4.5% total wall time. The cache value is a ceiling, not an
eager allocation. No schema, stored format, timeout, or correctness gate
changed.

For cold indexes below 50,000 supported files and for every incremental run,
secondary indexes remain live. Large cold indexes rebuild them transactionally:
other readers never observe a committed schema without the indexes, and a
rollback restores the prior indexes. Multi-row statements use at most 768 bind
parameters, below SQLite's legacy 999-variable limit.

## Full 500k receipt

The deterministic full corpus ran on the cache-ceiling commit
`123d4811bd67ebe8ca5f6386d512c3448656f373` and exited 0. The raw report is
`certification-500k.json`, SHA-256
`f767fd1b68a60ab85f38d93f73bfe935e2c247f98059ae3b1a8a8d7a8ddabc2f`.

| Metric | Result |
| --- | ---: |
| Physical / inventoried / parsed files | 520,101 / 500,101 / 500,100 |
| Parse errors | 0 |
| Nodes / edges / partitions | 2,174,361 / 1,696,005 / 501 |
| Generation | 398,440 ms |
| Inventory and cold index | 838,073 ms |
| Persistence (summed measured work) | 696,487 ms |
| Incremental modify/rename/delete | 21,797 ms; all true |
| Total gate time | **1,259,776 ms (20m 59.8s)** |
| Peak memory | 941,727,744 bytes (6.30%) |
| Verdict | **`passed: true`** |

This receipt proves the cache-ceiling commit, not a later sealed release. The
full benchmark must run again on the final sealed release SHA after every
subsequent release-affecting change.

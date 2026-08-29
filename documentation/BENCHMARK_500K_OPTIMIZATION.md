# 500k Index Optimization Ledger

Status: **full 500k gate passed on optimization commit; final release SHA rerun required**

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

## Selected implementation

V3 is the best measured safe variant. Compared with the baseline it reduces
persistence by 51.6% and complete index time by 50.5%. The cache value is a
ceiling, not an eager allocation. On the measured 14.9 GiB Windows host the
observed process peak is about 2.7% of physical memory.

For cold indexes below 50,000 supported files and for every incremental run,
secondary indexes remain live. Large cold indexes rebuild them transactionally:
other readers never observe a committed schema without the indexes, and a
rollback restores the prior indexes. Multi-row statements use at most 768 bind
parameters, below SQLite's legacy 999-variable limit.

## Full 500k receipt

The unchanged full deterministic corpus ran on clean commit
`c9ff3ef61e436d44d48d8aaac91d423f4d93a66c` and exited 0. The raw report is
`certification-500k.json`, SHA-256
`a49e0df468572e92c919fe01624652f400d1a039833b1e354b4b318f9edd611d`.

| Metric | Result |
| --- | ---: |
| Physical / inventoried / parsed files | 520,101 / 500,101 / 500,100 |
| Parse errors | 0 |
| Nodes / edges / partitions | 2,174,361 / 1,696,005 / 501 |
| Generation | 419,782 ms |
| Inventory and cold index | 949,687 ms |
| Persistence (summed measured work) | 686,379 ms |
| Incremental modify/rename/delete | 46,143 ms; all true |
| Total gate time | **1,418,402 ms (23m 38.4s)** |
| Peak memory | 1,075,535,872 bytes (7.19%) |
| Verdict | **`passed: true`** |

This receipt proves the optimization commit, not a later release. The full
benchmark must run again on the final sealed release SHA after every subsequent
release-affecting change.

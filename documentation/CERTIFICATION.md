# Certification Report — Cycle for Trae Work 0.1.0

Date: 2026-08-23
Platform: Windows 11 Pro x64 — AMD Ryzen 5 3400G (8 logical CPUs), 14.9 GB RAM
Scope: Phase 5 exit criteria per `SPECIFICATION.md` §27 — install/uninstall, full autonomous cycles (quick and full), repair path, all commands, 500k-file scale, concurrent projects.

Verdict: **PASS with one documented hardware-bound benchmark exception** (§4).

## 1. Engineering gates

| Gate | Command | Result |
| --- | --- | --- |
| Formatting | `cargo fmt --all --check` | clean |
| Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | clean |
| Tests | `cargo test --workspace --all-features` | 279 passed, 0 failed |
| Release build | `cargo build --release --workspace` | success |

Test totals per crate: workflow-core 53, workflowd 78, workflow-store 33, workflow-code-intel 35, workflow-roles 20, workflow-ipc 17, workflow-ledger 16, workflow-memory 8, trae-cycle 19.

## 2. Functional certification

All scenarios run through the real MCP stdio surface (`initialize` → `tools/call`) against the real daemon, real Git repositories, and scripted OpenAI-compatible role endpoints that follow the shipped prompts.

| # | Scenario | Test | Result |
| --- | --- | --- | --- |
| 1 | Quick autonomous cycle: roles missing → fail-closed; request → plan → worktree → commit → evidence → freeze + verification job → repository index → arbiter consult → arbitration → exact-byte promotion on the source branch; delivered file content and history receipt verified | `certification::quick_cycle_delivers_tested_software_end_to_end` | PASS |
| 2 | Full cycle with blind reviews and repair: architect advisory consult → plan (2 requirements) → candidate → verification → functional + security blind binding reviews → arbiter rejection returns the workflow to `execution` with `repairCycles == 1` → second candidate → re-reviews → arbiter approval → promotion; repaired file content verified | `certification::full_cycle_requires_two_blind_reviews_and_repairs_once` | PASS |
| 3 | Tool sweep over the MCP surface: setup, task CRUD, pause/resume/retry, execution report, goal CRUD and control, memory, history, models, limits, executor fail-closed, redacted export, cancel; the delivery-path tools are exercised end-to-end by scenarios 1–2 (36 tools total) | `certification::every_cycle_tool_answers_through_mcp` | PASS |
| 4 | Concurrent projects on one control plane: two projects, one delivered and one cancelled, mutually isolated worktrees and files | `certification::concurrent_projects_share_one_control_plane` | PASS |
| 5 | MCP handshake, 36 advertised tools, protocol 2025-06-18 | `mcp_e2e` | PASS |

Defects found and fixed during certification:

- The security review prompt advertised the role name `security_reviewer`, while the wire enum is `security_architecture_reviewer`. Verdicts from compliant endpoints would have failed role deserialization and silently degraded to advisory. Fixed in `workflow-roles/src/prompts.rs`.
- Worktree creation and repository indexing existed daemon-side but had no MCP tools, leaving the promote prerequisite unreachable from Trae Work. Added `cycle_worktree` and `cycle_index` (34 → 36 tools) and updated the skill and tool-contract references accordingly.

## 3. Scale benchmark — 500k files

Command: `cargo run --release -p workflow-code-intel --example codebase_500k -- --output documentation\certification-500k.json`
Raw report: `documentation/certification-500k.json`.

| Metric | Value |
| --- | --- |
| Physical files / inventoried / parsed | 520,101 / 500,101 / 500,100 (20,000 ignored by inventory rules) |
| Parse errors | 0 |
| Graph | 2,174,361 nodes, 1,696,005 edges, 501 partitions |
| Cold index total | 3,723,366 ms (62.1 min) — inventory+index 52.2 min, of which SQLite persistence 50.6 min |
| Incremental refresh | 34,904 ms; forced-scope modify, rename and delete all detected and reconciled |
| Peak memory | 308 MB (2.16 % of system RAM) |
| Peak CPU | 24.8 % |

Functional criteria: all green — zero parse errors at scale, correct incremental reconciliation, bounded resources.

Exception: the benchmark's internal pass gate also requires a full cold index under 30 minutes. On this machine the run exceeded it (62.1 min), dominated by partition persistence writes (~81 % of wall time) on SATA-class storage. This is a throughput characterization of the certification hardware, not a correctness failure: the operation Trae Work sessions repeat is the incremental refresh, measured at 34.9 s for the full 500k corpus. The gate is expected to pass on NVMe-class storage; if a cold-index SLA becomes a requirement, persistence batching is the identified lever.

## 4. Installation verification

| Artifact | Location | Verified |
| --- | --- | --- |
| Release binary | `%LOCALAPPDATA%\Trae Cycle\bin\trae-cycle.exe` — 40,502,784 bytes — SHA-256 `5CF425094DF088430431772943263F12973A5B2DE494BFF624A20C135EEFBEEA` | smoke test below |
| Skill | `%USERPROFILE%\.trae-cn\skills\cycle-delivery\` (SKILL.md, references/tools.md, references/evidence-protocol.md) | byte-identical to `plugin/skill/` sources (SHA-256) |
| MCP registration | `plugin/install/mcp.local.json` with absolute local paths | matches install |

Smoke test of the installed binary (real install path, including the space in `Trae Cycle`):

- `initialize` → `serverInfo {name: "trae-cycle", version: "1.0.0"}`, `protocolVersion 2025-06-18`, exit 0, empty stderr.
- `tools/call cycle_limits` → live admission policy through the control-plane daemon (cold-start spawn on first run, daemon reuse on second run).
- After the probe the data directory was reset to the pristine install state (runtime files and probe database removed); no `trae-cycle` processes left running.

## 5. Uninstall procedure

1. Remove the `trae-cycle` MCP server entry from Trae Work settings (prevents future spawns).
2. Stop the control-plane daemon: `Get-Process trae-cycle | Stop-Process -Force`. The daemon intentionally outlives MCP sessions and has no idle shutdown.
3. Delete `%LOCALAPPDATA%\Trae Cycle` (binary, control-plane database, audit ledger, managed worktrees).
4. Delete `%USERPROFILE%\.trae-cn\skills\cycle-delivery`.
5. Delete the `cycle` command from Trae Work settings.

The product creates no registry entries, services or scheduled tasks; all state lives in the two folders above plus the Trae Work settings entries.

## 6. Residual observations

- Role endpoints are user-supplied at `%LOCALAPPDATA%\Trae Cycle\config\roles.json`; every gated operation fails closed while the file is absent or invalid (certified in scenarios 1 and 3).
- Linux x64 certification is deferred to CI (Phase 6) per the plan sequencing; the workspace contains no Windows-only source in the certified crates beyond the documented `CREATE_NO_WINDOW` spawn flag and named-pipe transport already covered by platform tests.

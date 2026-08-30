# Cycle for Trae Work 1.0.1 Certification Matrix

Status: **BLOCKED — production release candidate, not authorized to publish**

This matrix is the version-bound execution ledger for `1.0.1`. A row is `PASS`
only when its named environment and boundary were exercised on the recorded
revision or exact artifact. Automated control-plane evidence never substitutes
for a Trae Work UI row. Evidence from `1.0.0` is historical and is not inherited.

## Candidate identity

| Field | Current value |
| --- | --- |
| Branch | `release/v1.0.1-production` |
| Latest remotely verified revision | `d5aa25bc95dee068f2e6891bf6d8342aacb6862c` |
| Trae Work Windows host | `0.1.54` |
| Windows runner | Windows 11 local; GitHub `windows-2025` |
| WSL runner | WSL2 Ubuntu 24.04 x64, ext4 clone; GitHub `ubuntu-24.04` |
| Rust | `1.97.1` with locked dependencies |
| macOS | **compatible but untested**; no v1 certification or support SLA |

## 1. Source, security, and configuration

| ID | Check | Windows | WSL | Evidence or blocking condition |
| --- | --- | --- | --- | --- |
| S1 | Format, clippy `-D warnings`, workspace tests, package contract | PASS | PASS | [CI run 33286348988](https://github.com/jannotix/trae-work-cycle-plugin/actions/runs/33286348988), exact `d5aa25b…` |
| S2 | Cargo license policy and RustSec audit | PASS | PASS | Same exact CI run; both independent jobs passed |
| S3 | Loopback roles work without a synthetic key or Authorization header | PASS | PASS | `workflow-roles` config and fake-endpoint tests; WSL ext4 replay passed |
| S4 | Non-loopback role endpoint without a key source fails closed | PASS | PASS | `remote_endpoints_without_key_sources_are_rejected` in both CI lanes |
| S5 | Nonpreapproved project command cannot run without exact ledgered consent | PASS | PASS | Unit/integration and MCP certification suites; manual Trae Work consent display remains row W7 |
| S6 | Public security, privacy, support, update, and rollback policies exist | PASS | PASS | `SECURITY.md`, `PRIVACY.md`, `SUPPORT.md`; package contract requires them |
| S7 | Private vulnerability reporting channel is live | PASS | N/A | GitHub repository setting enabled on 2026-08-30 |

## 2. Installation and native host surfaces

| ID | Check | Windows / Trae Work | WSL | Evidence or blocking condition |
| --- | --- | --- | --- | --- |
| I1 | Executable installs at a path without spaces | PARTIAL | PARTIAL | Windows `1.0.1` binary installed at `%LOCALAPPDATA%\TraeCycle\bin\trae-cycle.exe`, SHA-256 `343b19d6fdc1ebe55e5589ce0051477fb86aaa6d92a72a0084ddfcc6f1011ee0`; final signed artifact and final WSL archive pending |
| I2 | Extracted runtime archive contains binary, README, license, notice, and complete third-party notices | PARTIAL | PARTIAL | Windows deterministic unsigned rehearsal and WSL deterministic manual rehearsal passed; final signed/exact-SHA archives pending |
| I3 | Skill ZIP has root-level `SKILL.md`, `LICENSE`, and `NOTICE` and Trae Work accepts it | PARTIAL | N/A | `1.0.1` ZIP SHA-256 `86231a2208013ab6147ee26e3cd20df1a1f629fd03733e030639a5a952796ec8` passes package contract; Trae Work upload was proven only for the earlier `1.0.0` candidate |
| I4 | `cycle` Command is created from the shipped definition | PARTIAL | N/A | Command is present in Trae Work 0.1.54; exact final Skill/update session still pending |
| I5 | MCP entry starts the installed binary and exposes the complete tool catalog | PARTIAL | PARTIAL | Trae Work showed the local server healthy and exposed `cycle_consent`; current `1.0.1` restart is pending. CLI MCP smoke passed on Windows and WSL |
| I6 | Clean uninstall removes executable, Skill, Command, and MCP entry but preserves projects/data | BLOCKED | BLOCKED | Must run after quick/full/restart on final artifacts, then clean reinstall |

## 3. Trae Work workflow certification

| ID | Check | Automated control plane | Trae Work 0.1.54 UI | Evidence or blocking condition |
| --- | --- | --- | --- | --- |
| W1 | `/cycle setup` and `/cycle doctor` report the configured loopback roles healthy | PASS | BLOCKED | The old binary reported the now-fixed optional-auth defect. `1.0.1` is installed, but another active Windows Computer Use request invalidates every action before the rerun |
| W2 | Quick workflow traverses Skill → Command → MCP → role → verification → arbitration → promotion | PASS | BLOCKED | `quick_cycle_delivers_tested_software_end_to_end` passes Windows and WSL; UI run pending |
| W3 | Full workflow performs two blind reviews, rejection, repair, re-review, and approval | PASS | BLOCKED | `full_cycle_requires_two_blind_reviews_and_repairs_once` passes Windows and WSL; UI run pending |
| W4 | Two projects remain isolated under concurrent workflows | PASS | PENDING | Automated Windows/WSL test passed; UI isolation observation not yet recorded |
| W5 | Restart during a workflow resumes from durable state | PASS | BLOCKED | Daemon lifecycle/restart tests pass; close/reopen Trae Work and `/cycle:resume` pending |
| W6 | Trae Work update preserves MCP, Skill, Command, and durable state | N/A | BLOCKED | Requires an available Trae Work update or a documented same-version reinstall rehearsal |
| W7 | Exact verification command is displayed and runs only after user grants its single-use consent token | PASS | BLOCKED | Automated negative/positive path passes; manual prompt/approval/second-verify path pending |

## 4. Performance and release artifacts

| ID | Check | Result | Evidence or blocking condition |
| --- | --- | --- | --- |
| P1 | 500,100-source-file benchmark exits 0 with `passed: true` under the unchanged 30-minute SLA | PARTIAL | Passed on optimization commit `c9ff3ef…` in 1,418,402 ms with zero parse errors; must rerun after the final source/report commit |
| P2 | Windows runtime is Authenticode-signed and RFC 3161 timestamped | BLOCKED | No local code-signing certificate and no `windows-code-signing` environment secrets exist. The release workflow refuses unsigned bytes |
| P3 | Runtime archives are deterministic and preserve exact platform modes | PASS | Two Windows rehearsals produced identical SHA-256; WSL tar replay verified documents `0644` and executable `0755` |
| P4 | Full release inventory, manifest, checksums, SBOM, secret scan, and provenance all verify | PARTIAL | Local current-revision packaging passes without the final WSL/signed Windows pair; tag release workflow remains pending |
| P5 | Approval environments prevent unattended signing/publication | PASS | `windows-code-signing`: reviewer `jannotix`, branch `main` or tag `v*`; `production-release`: reviewer `jannotix`, tag `v*` only |
| P6 | Marketplace bundle is accepted and submitted | BLOCKED | Listing, permissions, data flow, policies, install recipe, logo, and checklist are prepared. External submission waits for final public assets and owner confirmation at action time |

## 5. Final gate order

1. Release the competing Windows Computer Use request, then rerun the pinned
   Trae Work UI rows W1–W7 with the `1.0.1` Skill and binary.
2. Complete restart, update/reinstall, uninstall, and clean reinstall on Windows;
   complete the equivalent native CLI/MCP lifecycle on WSL.
3. Supply a production code-signing identity as protected environment secrets,
   approve the signing job, and verify the timestamped signature after extraction.
4. Record the final results, commit the completed matrix, and freeze that SHA.
5. Rerun CI, the full 500k benchmark, clean Windows/WSL packaging and the
   non-publishing release workflow on that exact SHA.
6. Only then issue `AUTHORIZED TO PUBLISH`. Tagging, publishing, marketplace
   submission, and `PUBLIC RELEASE VERIFIED` remain separate later decisions.

Current verdict: **BLOCKED**.

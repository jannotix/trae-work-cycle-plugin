# Changelog

## 1.0.1 (Unreleased)

### Added

- Expiring, single-use verification-command consent bound to the workflow, frozen candidate, plan, gate, command vector, and worktree, with activation recorded in the audit ledger before execution.
- Deterministic extracted-archive smoke tests, root-level Skill packaging, Windows Authenticode signing and timestamp gates, complete runtime license material, public security/privacy/support policies, and a fail-closed marketplace submission kit.
- Source provenance through SSH-signed release tags, alongside the existing GitHub build-provenance attestations for the artifacts.

### Changed

- Windows installs use the no-space executable path `%LOCALAPPDATA%\TraeCycle\bin\trae-cycle.exe`; WSL uses the native `trae-cycle-wsl-x64.tar.gz` artifact with mode `0755`.
- Release automation is pinned, locked, exact-SHA, clean-checkout, inventory-complete, approval-protected, and verifies every checksum and provenance attestation before publication.
- Cold 500,100-source-file indexing now passes the unchanged 30-minute SLA while preserving zero parse errors, reconciliation correctness, and resource ceilings.
- Loopback role endpoints may omit authentication without receiving a synthetic bearer token; every non-loopback endpoint still requires an environment-variable or file key source.
- Windows and WSL certification wait for the daemon to publish both its IPC credential and matching PID before sending parallel workflow requests.

### Fixed

- An arbiter approval the control plane could not honour was refused with an error before anything was written: no arbitration row, no history event, and a workflow left sitting in arbitration. The next dispatch then produced the same verdict from the same inputs, so a run in which either independent reviewer rejected while the arbiter approved could not converge — and the chain the product exists to keep said nothing about why. The verdict is now recorded verbatim, named `arbitration_refused` in the audit trail with the reason and the repair target beside it, and routed to repair toward the target the rejecting reviewer asked for. One dispatch converges even when the arbiter is wrong. A failed mandatory gate under an approval is handled the same way, and always repairs toward execution.

  Ported from the same defect in Cycle for Claude Code, which found it during its first governed cycle where the two reviewers split. Trae Work had never reached that state, because no full cycle with two blind reviews has yet completed through the host — which is exactly the row this would have blocked.

- Verification now reads what a change *reaches*, not only what it touches. After the authorized write scopes are collected, the code graph is asked which files consume them, and the reached set is matched against the same layer rules — so a change to a configuration file that an interface file imports earns the browser and accessibility gates without anyone having edited an interface. This is promote-only by construction: adding paths to the set can insert a gate and has no way to remove one, so a wrong answer costs a proof that was not needed and never one that was. Depth is two — a consumer and its consumer — and a reached set wider than the ceiling reports `impact:high-fan-in` naming the most-consumed symbols instead of expanding into hundreds of gates.

  Uncertainty is recorded rather than passed over. "Nothing is affected" and "I cannot tell what is affected" are different claims, and reporting the first when the second is true is the failure this exists to avoid: a project the index does not hold produces `impact:unresolved` naming the reason and the command that fixes it. Neither reach gate is mandatory, because code intelligence is bound to delivery in this port and a first workflow reaches verification before its project has ever been indexed; refusing there would block every first cycle over an absence the delivery gate already catches. The skill now runs `cycle_index` during execution rather than at delivery, so the reach can resolve on the first cycle instead of only on repairs. Ported from Cycle for Claude Code 1.0.23.

- Candidate bytes were kept forever. Every file of every frozen candidate stayed in the control-plane database after its workflow completed or was cancelled, with nothing that reported the cost or gave it back, so a long-lived project's store only ever grew. `cycle_limits` takes an `operation`: `usage` reports what a project's candidates retain and what a prune would return, and `prune` — with `confirm: true` after explicit user approval — releases the retained bytes of completed and cancelled workflows' candidates. Nothing else goes: the candidate row, its manifest, every per-file digest and executable mode, the exact diff, the evidence recorded against it and every ledger entry naming it all stay, so what a candidate contained remains provable after its bytes are gone. A pruned candidate reports `payload_complete = 0` and refuses delivery by the same route as one whose payload never arrived. Bytes belonging to a workflow that is still running — through execution, verification, review, arbitration or an approval not yet promoted — are never touched. Ported from Cycle for Claude Code 1.0.23.

- Nothing told the session what to do when the control plane could not be reached. The nine ground rules stopped it from *claiming* a delivery without a receipt, but not from doing the work by hand when `cycle_start` failed — and in Trae Work the executor is the host session with every tool already in reach, so a missing MCP registration or a daemon that will not start is enough to produce files changed, an answer that says it is done, and a store holding no candidate, no gate and no record. The skill and the `cycle` command now stop, name whether the server is missing or failing and point at `/cycle doctor`, and say that doing the work ungoverned is not the remedy either. Found by checking Trae Work against Cycle for Claude Code 1.0.24, which hit this through an `--allowedTools` list that omitted the workflow tool.

- The arbiter saw the independent reviews only if the caller had put them in the request, so a binding verdict could be formed without ever being shown a rejection — which is how an approval over one gets produced in the first place. `cycle_role` with `arbiter_verdict` now requires `workflow_id`, reads the recorded reviews from the control plane's own store, and refuses the consultation when it has no workflow to read them from or when a full-mode candidate does not yet hold both. The record rides in the arbiter's system message beside the plane's prompt, so the caller's request still reaches the role that judges against it verbatim.

### Security

- Worktree isolation is explicitly not described as an operating-system sandbox. Nonpreapproved project commands cannot execute without an exact user consent receipt.
- Marketing and privacy material now disclose every configured network role boundary and no longer describe the product as fully local when remote role endpoints are used.
- Windows Authenticode signing is now conditional on a configured code-signing identity instead of an unconditional release gate. When secrets are present the runtime is signed and the signature enforced after extraction, exactly as before; when they are absent the release workflow emits a warning and publishes an unsigned runtime. This release ships unsigned, so its first run shows a SmartScreen warning, and every public surface — release notes, security policy, support policy, install recipe, README and marketplace listing — now says so. Artifact provenance rests on `SHA256SUMS.txt`, `MANIFEST.json` and the GitHub attestations. A signature that fails to verify still blocks publication; only its absence no longer does.

## 1.0.0 (2026-08-24)

### Added

- Phase 0: design specification covering architecture, MCP tool contracts, role model configuration, workflow lifecycle, verification gates, goal mode, resource admission and v1 acceptance criteria.
- Phase 1: Rust workspace forked from the shared Cycle control plane: workflow-core, workflow-ipc, workflow-store, workflow-ledger, workflow-memory, workflow-code-intel and workflowd.
- Phase 2: `trae-cycle` frontend crate: MCP stdio server exposing the 34 `cycle_*` tools with job-tracked long operations, fail-closed `roles.json` model configuration, on-demand daemon supervision, `serve`/`mcp`/`backup` CLI, a live admission `limits` control operation, and end-to-end MCP-to-daemon test coverage.
- Phase 3: `workflow-roles` crate: OpenAI-compatible HTTP client for the four read-only roles with structured outputs (binding `ReviewVerdict`/`ArbiterVerdict` parsing, advisory fallback), fail-closed key resolution, transient/permanent error classification, per-role token usage ledger, audit-trail recording, and the live `cycle_role` job tool; deterministic fake-endpoint test suite.
- Phase 4: Trae Work integration assets under `plugin/`: the `cycle-delivery` skill (SKILL.md plus tool-contract and evidence-protocol references), the `cycle` command definition covering the full `/cycle` routing surface including colon forms, and the MCP registration example with startup and run timeouts.
- Phase 5: Windows x64 certification. `cycle_worktree` and `cycle_index` MCP tools (36 total) closing the gap between the daemon delivery flow and the tool surface, security review prompt role name aligned to the wire enum, MCP end-to-end certification suite (autonomous quick cycle, full cycle with blind reviews and one repair round, tool sweep, concurrent projects), 500k-file code-intelligence benchmark with incremental refresh report, local installation with smoke-tested binary and global skill, uninstall procedure, and the certification report.
- Phase 6: Release. Allowlist packaging pipeline (`tools/package.ps1`) producing the plugin package and skill archive with SHA256SUMS, CycloneDX SBOM, third-party notices and a provenance MANIFEST whose revision must equal the source revision, enforced by a verify mode; permissive SPDX license gates both locally and in CI (cargo-deny); CI on Windows and Linux (format, lint, tests, licenses) and a tag-driven release workflow with per-platform binaries, smoke tests, build-provenance attestations and verified release publication; public documentation set (user manual, commands reference, threat model, roadmap).

### Changed

- Product identity renamed for Trae Work: IPC auth domain, named pipe namespace, ledger hash and checkpoint domains, digest domains, delivery journal directory, data directories (`Trae Cycle`, `trae-cycle`), managed browser tool identity and repository URL.
- Durable state guard now reports host installations generically instead of naming a specific host product.

### Removed

- Legacy delivery journal migration path retained from the previous product line. Cycle for Trae Work starts from empty durable state, so pre-fork journal recovery is unreachable code.

# Changelog

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

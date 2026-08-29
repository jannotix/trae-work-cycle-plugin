# Cycle for Trae Work — Marketplace Listing Pitch

Prepared for the TRAE Work plugin marketplace team. One page, factual.

## Proposal

List Cycle for Trae Work as a **development-tools plugin** in the TRAE Work marketplace. It fills a category the current catalog does not cover: governed, evidence-gated software delivery.

## One-line description

Evidence-gated software delivery for TRAE Work: five isolated roles, blind reviews, arbiter approval and exact-byte delivery through a local MCP control plane.

## What users get

- `/cycle run quick|full` — from an immutable user request to delivered, verified bytes in the user's real Git repository;
- Two blind reviews plus an independent arbiter bound to the original request — the executor can never approve its own work;
- Real verification gates (project's own test/build/browser commands), candidate freeze, exact-byte promotion, bounded repair (5 cycles), hash-chained local audit ledger;
- Single-role consultations (`/cycle architect`, `/cycle reviewer`, `/cycle security`, `/cycle arbiter`), multi-milestone goals, project memory, `/cycle:resume` recovery;
- Model-agnostic: each of the four read-only roles runs on any user-configured OpenAI-compatible endpoint; the executor stays on the model selected in TRAE Work.

## Why it fits the marketplace model

- **Local-first**: certified Windows x64 and WSL2 x64 native runtimes, with macOS declared **compatible but untested**; no Cycle cloud service, account or telemetry;
- **Standard surfaces only**: MCP server (stdio, protocol 2025-06-18, 36 tools), one skill (`cycle-delivery`), one command (`cycle`) — exactly the three extension points TRAE Work already supports;
- **Distribution-ready**: GitHub releases with SHA256SUMS, CycloneDX SBOM, provenance manifest and signed build attestations; the listing manifest draft is in this repository (`marketplace/manifest.json`).

## Quality evidence

- Certification report (Windows x64): autonomous quick and full cycles, repair path, all tools, concurrent projects — green;
- 500k-file code-intelligence benchmark: 0 parse errors, incremental refresh 35 s, peak memory 308 MB;
- CI on Windows and Linux: format, clippy `-D warnings`, 279 tests;
- Security posture documented in `documentation/THREAT_MODEL.md`; secrets are referenced via environment variables or files and never stored or logged.

## Licensing

Product: FSL-1.1-MIT (fair-source; automatic MIT conversion two years after each release). Skill and documentation text contributed to community catalogs under the catalog's license.

## Integration requirements for a one-click listing

Today the plugin registers through three manual steps (MCP JSON, skill folder, command). A marketplace listing would need either:

1. an installable bundle format the TRAE Work client can unpack (binary + skill + command in one package), or
2. marketplace support for a local binary command with per-user paths.

Both are fully automatable from our release pipeline; no product changes are required on our side.

## Links

- Repository: https://github.com/jannotix/trae-work-cycle-plugin
- Releases: https://github.com/jannotix/trae-work-cycle-plugin/releases
- Author: Gianluca Iannotta (GitHub: jannotix)

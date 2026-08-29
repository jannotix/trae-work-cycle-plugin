# Cycle for Trae Work v1.0.1 Production Release Plan

Status: **BLOCKED — implementation and exact-artifact certification are incomplete**

This plan is the release ledger for Cycle for Trae Work v1.0.1. A checked box is
not release evidence by itself. Every completed task must name its exact clean
Git revision, commands, platform, artifact hashes, and retained receipts.

## Release verdicts

The only valid release verdicts are:

- `BLOCKED` — at least one required gate is incomplete or failed.
- `AUTHORIZED TO PUBLISH` — all pre-publication gates passed on the exact clean
  revision and publication has explicit owner authorization.
- `PUBLIC RELEASE VERIFIED` — the published assets passed clean-install and
  rollback verification from their public URLs.
- `WITHDRAWN — ROLLBACK REQUIRED` — publication occurred but public verification
  failed or a material release defect was found.

`AUTHORIZED TO PUBLISH` never implies `PUBLIC RELEASE VERIFIED`.

## Platform contract

| Surface | v1 status | Required evidence |
| --- | --- | --- |
| Windows 10/11 x64 CLI and Trae Work Desktop local environment | Certified | Clean-machine install, extracted Windows archive, MCP registration, skill and command activation, quick/full/repair cycles, restart, update, uninstall, signature, and exact hashes |
| WSL2 Ubuntu x64 CLI/MCP runtime | Certified | Native ext4 checkout, extracted Linux archive with executable mode, quick/full/repair cycles, restart, update, uninstall, and exact hashes |
| macOS | **compatible but untested** | Static/source compatibility only. No macOS result may replace a Windows or WSL receipt, and v1 carries no macOS support SLA |
| Native Linux desktop outside WSL | Not a v1 Trae Work certification target | The Linux runtime artifact is certified for the WSL lane only |

The user-facing wording must be exactly `compatible but untested` for macOS.

## What is adopted from Cycle for Claude Code

The Claude Code variant is a reference for release discipline, not an API or
runtime dependency. Trae Work keeps its native MCP, Skill, and Command surfaces.

| Claude Code practice | Trae Work decision |
| --- | --- |
| Certification matrix with row-to-evidence traceability | Adopt for Windows and WSL, with every manual result bound to the exact product and Trae Work versions |
| Host loads both source and packaged artifacts | Adopt against Trae Work Desktop on Windows and the extracted native CLI/MCP archive in WSL |
| Reproducible ZIP writer and explicit artifact allowlist | Adopt, extended with executable Unix modes and source-revision binding |
| Packaged-artifact smoke tests | Adopt; all smoke tests run after extraction in a second directory |
| Honest `SECURITY.md` description of unsandboxed execution | Adopt and strengthen: discovered commands require policy approval or an explicit user consent receipt |
| Version-bound manual certification results | Adopt; stale results are blocking, never inherited by a later version |
| Plugin-specific hooks and Claude agent manifests | Do not copy; Trae Work has no equivalent guaranteed hook contract |
| Session-model inheritance and Claude provider routing | Do not copy; Trae Work roles remain user-configured OpenAI-compatible endpoints |

## Non-negotiable invariants

1. One task produces one atomic commit and an independently inspectable receipt.
2. The release candidate is built from a fresh clone of one exact clean full SHA.
3. Cargo commands use the committed lockfile and pinned Rust toolchain.
4. Windows and WSL consume source from the same SHA. Platform archives may
   differ, but shared plugin/skill/command bytes must be identical.
5. No receipt survives a source, lockfile, workflow, packaging, or documentation
   change that affects the release artifact or its claims.
6. A failed mandatory test, benchmark, signature, host, or artifact check blocks
   downstream publication. Timeouts and security checks are not relaxed to pass.
7. Release evidence excludes credentials, role prompts, private endpoints,
   absolute personal paths, raw secrets, and unbounded process output.
8. The tag `v1.0.0` is immutable and must not be moved or reused.
9. A public GitHub Release is not marketplace acceptance.

## Canonical release inventory

The final manifest must bind every item by name, version, byte length, SHA-256,
source revision, source tag, platform, and provenance identity:

1. `trae-cycle-windows-x64.zip`
2. `trae-cycle-wsl-x64.tar.gz`
3. `cycle-delivery-skill-1.0.1.zip`
4. `trae-work-cycle-plugin-1.0.1.zip`
5. `SBOM.cdx.json`
6. `THIRD-PARTY-NOTICES.md`
7. `SHA256SUMS.txt`
8. `MANIFEST.json`

The skill archive has `SKILL.md` at its root. Each native runtime archive carries
the product license, third-party notices, README, and its platform executable.

## Task ledger

### T00 — Freeze the release contract

- Record this platform matrix, artifact inventory, gates, and rollback model.
- Compare only transferable Claude Code practices and preserve Trae Work-native
  extension points.
- Acceptance: clean diff, plan structure check, one documentation-only commit.

### T01 — Correct installation and archive behavior

- Use a Windows executable path with no spaces.
- Generate the root-level Skill archive expected by Trae Work upload.
- Preserve executable mode in the WSL archive.
- Test every archive after extraction into a second directory.
- Reject dirty source trees and build package inputs from the committed revision.

### T02 — Gate verification commands with consent

- Define a versioned preapproved command policy for deterministic build, format,
  lint, test, and audit programs with bounded argument shapes.
- Any command outside that policy produces a consent request and cannot execute
  until a matching, single-use user receipt is recorded.
- Bind consent to project, normalized command vector, working directory,
  candidate digest, and expiry.
- Deny shells, interpreters with inline code, deployment/publish/destructive
  verbs, and path escapes by default.
- Update the threat model, manual, Skill, Command, and marketing claims.

### T03 — Make CI and release fail closed

- Pin Rust, third-party Actions, and release tooling.
- Use `--locked` for every Cargo build, test, metadata, audit, and package step.
- Require format, Clippy, tests, advisories, licenses, package validation,
  extracted-artifact smoke, and secret scan before release.
- Build release assets in clean platform jobs, seal them once, and verify every
  attestation and checksum before an approval-protected publish job.
- A tag/version mismatch, unsigned required binary, dirty tree, missing asset,
  unverified attestation, or stale receipt blocks publication.

### T04 — Pass the 500k benchmark

- Baseline the existing 500,100-source-file deterministic corpus.
- Keep the 30-minute cold-index limit, zero parse errors, correct incremental
  reconciliation, and resource ceilings unchanged.
- Test one persistence hypothesis per variant, preserve correctness, retain a
  variant ledger, and promote only a repeated best measured safe variant.
- Acceptance: the raw final report contains `"passed": true` on the final SHA.

### T05 — Certify Windows and WSL

- Pin the Trae Work Desktop version used for Windows certification.
- Drive the real Windows host through Skill upload, Command creation, MCP
  registration, `/cycle setup`, `/cycle doctor`, quick/full/repair flows,
  restart, update, uninstall, and clean reinstall.
- Run the same CLI/control-plane behavioral matrix natively in WSL2 from ext4.
- Bind every manual row to product version, host version, date, SHA, and artifact.
- Record macOS only as `compatible but untested`.

### T06 — Complete signing, compliance, privacy, and support

- Authenticode-sign and timestamp the Windows executable before sealing it.
- Include complete redistributable license and attribution material in each
  runtime archive and give the Skill an explicit license.
- Publish `SECURITY.md`, supported-version policy, private vulnerability channel,
  privacy/data-flow disclosure, support policy, update and rollback instructions.
- Remove claims that the product is fully local when configured role endpoints
  receive candidate material over a network.

### T07 — Marketplace bundle and submission

- Produce a one-install listing bundle or an explicit marketplace install recipe
  accepted by the current Trae Work catalog contract.
- Validate logo, description, permissions, data flow, platform labels, license,
  support links, and artifact URLs.
- Submit only the exact release candidate; marketplace review feedback requires
  a new commit and full recertification.

### T08 — Seal the release candidate

- Start from a fresh clone of the candidate SHA in a new directory.
- Re-run source, package, security, benchmark, Windows, WSL, signing, SBOM,
  checksum, and provenance gates.
- Copy/download artifacts to a second directory and independently verify hashes,
  archive contents, signatures, modes, and runtime identity.
- Acceptance verdict: `AUTHORIZED TO PUBLISH`.

### T09 — Publish and verify

- Create a new annotated, signed `v1.0.1` or later tag; never move `v1.0.0`.
- Publish only the sealed bytes approved in T08.
- Download every public asset by URL, verify hashes/attestations/signatures, and
  repeat clean-install smoke on Windows and WSL.
- Success verdict: `PUBLIC RELEASE VERIFIED`.
- Failure verdict: `WITHDRAWN — ROLLBACK REQUIRED`, followed by asset withdrawal
  and the documented rollback procedure.

## Current gate state

| Task | State |
| --- | --- |
| T00 | Completed — `78a0de3` |
| T01 | Implemented locally — Windows extracted-archive smoke passed; WSL runner receipt pending |
| T02–T09 | Not started |

The existence of v1.0.0, a green unit suite, or a corrected plan does not change
this release from `BLOCKED`.

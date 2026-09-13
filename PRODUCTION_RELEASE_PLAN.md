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
| Windows 10/11 x64 CLI and Trae Work Desktop local environment | Certified | Clean-machine install, extracted Windows archive, MCP registration, skill and command activation, quick/full/repair cycles, restart, update, uninstall, and exact hashes. An Authenticode signature is required evidence only when a code-signing identity is configured; otherwise the unsigned state is required disclosure |
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
6. A failed mandatory test, benchmark, host, or artifact check blocks downstream
   publication. Timeouts and security checks are not relaxed to pass. Windows
   Authenticode signing is mandatory whenever a code-signing identity is
   configured and a signature that fails to verify always blocks; an absent
   identity does not block, but publishing an unsigned runtime without saying so
   does.
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

- Authenticode-sign and timestamp the Windows executable before sealing it when a
  code-signing identity is configured. Without one, publish unsigned and disclose
  it in the release notes, the security policy, the support policy, the install
  recipe and the marketplace listing.
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

- Create a new annotated tag `v1.0.1` or later, signed with the maintainer's SSH
  signing key (`gpg.format = ssh`); never move `v1.0.0`. The tag signature proves
  the provenance of the source; the GitHub attestations prove the provenance of
  the built artifacts. Neither replaces the other, and neither is Authenticode.
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
| T01 | Implemented — `9e65034` plus runtime-material/mode hardening; Windows extracted-archive smoke and deterministic replay passed, WSL ext4 archive/mode/smoke replay passed; final signed/exact-SHA pair pending |
| T02 | Implemented — `67890d4` plus keyless-loopback correction `85d6afa`; policy, expiring single-use ledgered consent, runner boundary and MCP quick/full/repair/concurrency certification passed on Windows and WSL |
| T03 | Implemented — `a9a8726` plus release/signing follow-ups; exact CI run 33286348988 passed Windows 2025, Ubuntu 24.04, RustSec, license and package contracts on `d5aa25b…`; tagless release-candidate workflow still requires default-branch integration and signing secrets |
| T04 | Passed on `123d481`: raw receipt `passed: true`, total 1,259,776 ms, 898.1 MiB peak, 0 parse errors; final sealed-SHA rerun remains mandatory |
| T05 | Unblocked, execution pending — Trae Work Desktop 0.1.61 is pinned after a real in-place update from 0.1.54, which preserved the task, Command, MCP registration and data directory. The ten-day UI block had two causes, both now identified and neither in the logic of this product. First, the registered MCP entry pointed at the no-space path while the bytes actually installed there were the 1.0.0 build left at the legacy space-bearing path; the head build was installed on 2026-09-12. Second, the host binds the tool catalogue of a task when that task is created, so the pre-existing task kept reporting no `cycle_*` tool even after the host had loaded the server with its full catalogue. In a task created afterwards, `/cycle setup` returned a live control-plane report on the first attempt (2026-09-13). W2-W7 are therefore executable; the two earlier diagnoses recorded against them, a clipped prompt footer and a broken host MCP subsystem, are both withdrawn |
| T06 | Implemented — policies, license-bearing archives, private vulnerability reporting, signing script and approval environments complete. Authenticode is now conditional on configured secrets, and every public surface discloses the unsigned runtime; a production signing identity remains desirable but no longer gates publication |
| T07 | Prepared — manifest, permissions/data flow, install recipe, logo and checklist complete; external marketplace submission awaits final public assets and action-time owner confirmation |
| T08–T09 | Blocked — final UI receipts, final-SHA 500k rerun, sealed release candidate, publication approval and public verification remain mandatory |

## Host constraints and how this plan adapts to them

Trae Work is not Claude Code, and a gate this port cannot execute the way its
reference does is adapted rather than dropped or quietly marked done. What the
philosophy requires is preserved in every case: a claim is made only where a
tool result backs it, evidence names the exact revision and host that produced
it, and what was not observed is recorded as not observed.

| Host constraint | Measured on | Adaptation, and what it preserves |
| --- | --- | --- |
| The tool catalogue of a task is bound when the task is created. A task created before a server loads keeps reporting no tools for its whole life, whatever the host has loaded since | 2026-09-13: the log records the server loading with its full catalogue at 14:32:53, while the task created before it still answered that no `cycle_*` tool was present; a task created afterwards reached the control plane on the first attempt | Every UI row must be executed in a task created after the binary, MCP registration and Skill under test were installed, and the row records that creation time. This is stricter than the reference, not weaker: it closes a path by which a stale session could produce evidence about bytes it never loaded |
| A host update cannot be summoned on demand, so W6 is observable only when the vendor ships one | The 0.1.54 to 0.1.61 update on 2026-09-02 | W6 stays an opportunistic gate. What was observed at that update is recorded as fact; the rest is recorded as pending the next update, and the release is not held on the schedule of a third party. The disclosure obligation is unchanged: the matrix must say which half was observed |
| No code-signing identity exists, and the CA/Browser Forum has required hardware-held keys since June 2023 | The conditional signing step in the release workflow | Signing is enforced whenever an identity is configured and never simulated when one is not. Publishing unsigned does not block; publishing unsigned without saying so on every public surface does. Already implemented |
| Cycle cannot read Trae Work provider credentials, so the four read-only roles cannot be certified against whatever model the host itself uses | Product boundary, by design | The real-provider gate runs against an endpoint the operator configures in `roles.json`, and the receipt records the endpoint class rather than the key. Certifying the roles against the provider of the host is not adapted but refused: it would require reading a credential this product promises never to touch |
| Marketplace acceptance is the decision of a third party | T07 | The bundle, recipe and disclosures are prepared and verifiable locally; submission stays an owner action at action time. A GitHub Release is never recorded as marketplace acceptance |

## Open work not yet in the ledger above

These came out of the adversarial comparison against Cycle for Claude Code and
have been tracked only in conversation until now, which is the same
record-keeping failure the certification matrix was just repaired for.

| ID | Work | Why it is open |
| --- | --- | --- |
| R1 | Execute W2-W7 in a task created after the head install | The host condition that blocked them is understood and cleared; nothing has been run yet |
| R2 | Complete I6: uninstall, clean reinstall, and the equivalent WSL lifecycle | Never executed on any revision |
| R3 | One full cycle against a real configured provider rather than the loopback endpoint | Every role result on record comes from the certification loopback |
| R4 | Observe the consent prompt, approval and second verification in the interface | Only the automated negative and positive paths are on record (W7) |
| R5 | Confirm write-scope enforcement refuses an out-of-scope write on the live host | Proven in tests; never observed through the host |
| R6 | Audit the delivery path for panics reachable from operator input | `panic = abort` in release means a reachable panic is a lost workflow, not an error |
| R7 | Rerun the 500k benchmark on the sealed revision | The passing receipt names a revision that is now far behind |

## Defects ported from Cycle for Claude Code (2026-09-10)

Cycle for Claude Code found these by running governed cycles, not by reading
code. Each was then checked against this port and fixed where it applied. All
five landed on `main`, each with CI green on Windows 2025 and Ubuntu 24.04.

| ID | Change | Commit |
| --- | --- | --- |
| I1 | A refused arbitration is recorded and routed to repair instead of thrown away. An approval contradicting a live reviewer rejection left no arbitration row, no history event and a workflow stuck in `arbitration`, so the next dispatch reproduced the same verdict forever | `beb95d7` |
| I2 | The control plane hands the arbiter the recorded reviews rather than trusting the caller to include them, and refuses the consultation when it cannot. `cycle_role`/`arbiter_verdict` now requires `workflow_id` | `3c003b5` |
| I3 | Candidate file bytes of completed and cancelled workflows can be released through `cycle_limits` `usage`/`prune`, keeping every row, digest, evidence record and ledger link | `4170718` |
| I5 | Ground rule 10: the session stops when the control plane cannot be reached instead of doing the work by hand. Two of the three defects checked did not apply to this port — this one did, and it was **observed working on the live host** on 2026-09-12 | `cdceadd` |
| I4 | Verification reads what a change reaches through the code graph, not only what it touches; `impact:unresolved` and `impact:high-fan-in` record what it cannot answer | `19a16d2` |

Not ported, with reason: the delivery commit message that named zero gates has
no counterpart, because this port does not commit on delivery; telling a
never-started delivery from an aborted one is already handled by
`delivery_recovery_required`; archive reproducibility was already correct.

These twenty-one commits since `d5aa25b…` invalidate the certification evidence
recorded against earlier revisions. That is recorded at the top of
`documentation/CERTIFICATION_V1.0.1.md` and must be cleared before any seal.

The existence of v1.0.0, a green unit suite, or a corrected plan does not change
this release from `BLOCKED`.

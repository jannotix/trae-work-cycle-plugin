# Cycle for Trae Work 1.0.1 Certification Matrix

Status: **BLOCKED — production release candidate, not authorized to publish**

This matrix is the version-bound execution ledger for `1.0.1`. A row is `PASS`
only when its named environment and boundary were exercised on the recorded
revision or exact artifact. Automated control-plane evidence never substitutes
for a Trae Work UI row. Evidence from `1.0.0` is historical and is not inherited.

> **Evidence invalidated by source change (2026-09-13).** Twenty-one commits
> landed between `d5aa25b…`, the revision most rows below were exercised on, and
> `c538c7e…`, the current head. Seven of them changed control-plane behaviour:
> arbitration recording and repair routing, the arbiter's review payload, candidate
> retention, reach-driven gate discovery, and the skill's ground rules. By this
> matrix's own rule, every row whose evidence names an earlier revision no longer
> covers the shipped code and must be re-executed before any seal. The rows are
> left at their recorded results rather than blanked, because what was proven then
> is still a fact about then — but none of it is a fact about `c538c7e…`.

## Candidate identity

| Field | Current value |
| --- | --- |
| Branch | `main` and `release/v1.0.1-production`, identical at `c538c7e…` since the 2026-09-10 merge |
| Current local candidate revision | `c538c7e9d8949647c57d27d1809d080c572c3e48`; any later source change invalidates prior evidence |
| Latest local runtime and WSL evidence | `b9b48a2441692c67d5e7032292db6d076ee85b38` — **stale**, 21 commits behind the head |
| Latest remotely verified revision | `c538c7e9d8949647c57d27d1809d080c572c3e48` ([CI green on Windows 2025, Ubuntu 24.04, RustSec and the license gate](https://github.com/jannotix/trae-work-cycle-plugin/actions)); the rows below still cite `d5aa25b…` and have not been re-executed |
| Trae Work Windows host | `0.1.61` (real in-place update from `0.1.54` on 2026-09-02) |
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
| I1 | Executable installs at a path without spaces | PARTIAL | PARTIAL | Windows binary installed at `%LOCALAPPDATA%\TraeCycle\bin\trae-cycle.exe`. On 2026-09-11 the installed bytes were found to be the `1.0.0` build at the legacy space-bearing path, not the hash this row previously recorded; the current head build SHA-256 `66e84eaf6515cdfc8ee95c08730552da80faab19be84f37778973550261f63be` (from `c538c7e…`) was installed at the no-space path and the legacy install left in place. WSL candidate archive SHA-256 `7ee8fccd703186c8785e1483384f337fe4998b321171becb17965bd1777330c5` is equally stale; final exact-SHA pair pending |
| I2 | Extracted runtime archive contains binary, README, license, notice, and complete third-party notices | PARTIAL | PARTIAL | Windows candidate built at `05daf813…`, SHA-256 `4feb59a5cb74e426268282870ca454b23cc36fb7a7d41260c1c637887caea97b`, passed extraction and MCP smoke; it is deliberately unsigned. The earlier WSL candidate archive also passed extraction and MCP smoke. Final signed Windows and exact-SHA WSL archives remain pending |
| I3 | Skill ZIP has root-level `SKILL.md`, `LICENSE`, and `NOTICE` and Trae Work accepts it | PARTIAL | N/A | `1.0.1` ZIP SHA-256 `86231a2208013ab6147ee26e3cd20df1a1f629fd03733e030639a5a952796ec8` passes package contract; the active supported local Skill installation now has source `SKILL.md` SHA-256 `a44b45131cc2d57916028080271bad73f8a17b7d7ea433acab247af2a56a5658`; exact Marketplace-upload revalidation remains pending |
| I4 | `cycle` Command is created from the shipped definition | PARTIAL | N/A | The existing Command invoked an explicit `cycle_setup` MCP tool call after the host update; command-definition and final-Skill session revalidation remain pending |
| I5 | MCP entry starts the installed binary and exposes the complete tool catalog | PARTIAL | PARTIAL | Post-update explicit `cycle_setup` returned protocol v1/schema v18 and all configured loopback roles. CLI MCP smoke passed from extracted Windows and WSL candidate archives; complete host-catalog observation is pending |
| I6 | Clean uninstall removes executable, Skill, Command, and MCP entry but preserves projects/data | BLOCKED | BLOCKED | Must run after quick/full/restart on final artifacts, then clean reinstall |

## 3. Trae Work workflow certification

> **Resolved, 2026-09-13.** The condition that held every UI row below was
> found and cleared, and it was not in this product. Two causes compounded.
>
> The bytes installed at the registered no-space path were the `1.0.0` build: the
> runtime had been unpacked at the legacy space-bearing path, so the entry Trae
> Work launched and the binary under certification were not the same file. The
> head build was installed on 2026-09-12.
>
> The host then binds the tool catalogue of a task when that task is created. The
> session log records the server loading at 14:32:53 with its complete catalogue
> — `final result: 4 extensions: mcp.config.usrlocalmcp.trae-cycle-cert(tools=
> [cycle_setup,cycle_doctor,cycle_start,…` — while the task created before it went
> on answering that no `cycle_*` tool was present. In a task created after the
> install, `/cycle setup` returned a live control-plane report on the first
> attempt.
>
> Both earlier diagnoses are withdrawn. The clipped prompt footer described a
> symptom seen while the session held no tools at all. The reading that the host
> MCP subsystem never reaches a working state was refuted by the same log that
> suggested it: four servers were filtered out as `stopped`, and Cycle was not
> among them. Neither cost a line of product code, and both sent the
> investigation the wrong way — the first for ten days.
>
> W2–W7 are executable. None has been run.

| ID | Check | Automated control plane | Trae Work 0.1.61 UI | Evidence or blocking condition |
| --- | --- | --- | --- | --- |
| W1 | `/cycle setup` and `/cycle doctor` report the configured loopback roles healthy | PASS | PASS | Re-executed on the live host on 2026-09-13 at 14:40, in a task created after the head build was installed: the reply named control plane `TraeCycleCert 1.0.1`, protocol 1, schema `read_write` v18 writable, Git `2.55.0.windows.5`, and all five roles configured — executor on the current Trae Work model, arbiter, architect, functional_reviewer and security_reviewer on the certification loopback — and stated the project was ready for a cycle. This supersedes the withdrawn 2026-09-02 row, whose result the 2026-09-12 attempt had contradicted. `/cycle doctor` through the interface is not yet re-executed on this revision |
| W1a | The session refuses to fake governed work when the control plane is unreachable | N/A | PASS | Twice on 2026-09-12, with no `cycle_*` tool present, the session named the missing tools, quoted the skill's rule that "a claim is valid only when it is backed by a tool result", declined to report `cycle_setup` as run, stated what would fix it, and offered the ungoverned alternative explicitly rather than performing it silently. This is ground rule 10, added in `cdceadd…`, observed on the live host — and the only Trae Work UI evidence this release has collected |
| W2 | Quick workflow traverses Skill → Command → MCP → role → verification → arbitration → promotion | PASS | PARTIAL | Run on the live host on 2026-09-14 in a task created after the head install, against the isolated `trae-ui` certification fixture, which had a green two-test baseline and a clean tree. Observed through the interface: mode armed; the request captured verbatim as the immutable original, `requestDigest a144bb12f48938…`; architecture plan accepted; isolated worktree created; the change implemented and **three tests passing, including the new one**; `cycle_freeze` refusing a candidate while the worktree was dirty, then accepting it on a clean commit; candidate `01a09cd7-ee34-7b93…` frozen with exactly two files in the diff; state `verification`. Not observed: arbitration and promotion. The run stopped at the consent gate on the defect recorded in `b620f75`, which is fixed but not yet re-run end to end |
| W3 | Full workflow performs two blind reviews, rejection, repair, re-review, and approval | PASS | PENDING | `full_cycle_requires_two_blind_reviews_and_repairs_once` passes Windows and WSL; UI run pending |
| W4 | Two projects remain isolated under concurrent workflows | PASS | PENDING | Automated Windows/WSL test passed; UI isolation observation not yet recorded |
| W5 | Restart during a workflow resumes from durable state | PASS | PENDING | Daemon lifecycle/restart tests pass; close/reopen Trae Work and `/cycle:resume` pending |
| W6 | Trae Work update preserves MCP, Skill, Command, and durable state | N/A | PARTIAL | A real 0.1.54 → 0.1.61 update preserved the task, Command, MCP registration and data directory; the current Skill was then updated from the verified 1.0.1 ZIP. Restart/reload and workflow-resume capture remain pending |
| W7 | Exact verification command is displayed and runs only after user grants its single-use consent token | PASS | PARTIAL | Automated negative/positive path passes. Observed live on 2026-09-14: both verification commands displayed exactly as they would be invoked, each with its own 64-hex single-use token and a stated 15-minute expiry, the session declaring it would not call `cycle_consent` before explicit approval, and the plane **refusing a blanket "approve both"** — the gate rejecting an approval that does not name its command is the behaviour this row exists to check. Not observed: a token actually consumed and the command running under it. That half was blocked by the defect in `b620f75`: a `confirm` sent as a string was refused with the message used for a missing approval, so no wording of user approval could clear it and the tokens expired |

> **What the first live quick cycle found, 2026-09-14.** The run did what this row
> exists for: it found a production defect on the consent path that no unit test
> had. `cycle_consent` read `confirm` with `as_bool`, so a caller that sent the
> string `"true"` was refused with the same sentence used for a caller that sent
> nothing — *this command requires confirm: true after explicit user approval*. A
> model caller reads that as the user approval being insufficient and goes back to
> ask for a stronger one; the payload never changes, the refusal never changes, and
> the single-use tokens expire mid-loop. The most security-sensitive gate in the
> product had a state no correct user action could clear. Fixed in `b620f75`, which
> accepts either spelling and separates the three real refusals: absent, declined,
> malformed. Verified against the installed binary before and after.

## 4. Performance and release artifacts

| ID | Check | Result | Evidence or blocking condition |
| --- | --- | --- | --- |
| P1 | 500,100-source-file benchmark exits 0 with `passed: true` under the unchanged 30-minute SLA | PARTIAL | Passed on cache-ceiling commit `123d481…` in 1,259,776 ms with an 898.1 MiB peak and zero parse errors; must rerun after the final source/report commit |
| P2 | Windows runtime is Authenticode-signed and RFC 3161 timestamped | NOT CLAIMED FOR THIS RELEASE | No code-signing identity exists. Signing is now conditional: the release workflow signs and enforces the signature when `WINDOWS_CODE_SIGNING_CERTIFICATE_*` secrets are present, and otherwise emits a workflow warning and publishes an unsigned runtime. `1.0.1` therefore ships unsigned, first run shows SmartScreen, and artifact provenance rests on `SHA256SUMS.txt`, `MANIFEST.json` and the GitHub attestations. Signing becomes a mandatory gate again as soon as an identity is provisioned. Public wording must disclose the unsigned state |
| P3 | Runtime archives are deterministic and preserve exact platform modes | PASS | Two Windows rehearsals produced identical SHA-256; WSL tar replay verified documents `0644` and executable `0755` |
| P4 | Full release inventory, manifest, checksums, SBOM, secret scan, and provenance all verify | PARTIAL | The unsigned Windows candidate built at `05daf813…` passed extracted MCP smoke and release-secret scan. Full inventory, manifest, SBOM/provenance and the final WSL asset remain mandatory on the sealed SHA. The Windows asset is no longer required to be signed, but must match its manifest entry and attestation |
| P5 | Approval environments prevent unattended signing/publication | PASS | `windows-code-signing`: reviewer `jannotix`, branch `main` or tag `v*`; `production-release`: reviewer `jannotix`, tag `v*` only |
| P6 | Marketplace bundle is accepted and submitted | BLOCKED | Listing, permissions, data flow, policies, install recipe, logo, and checklist are prepared. External submission waits for final public assets and owner confirmation at action time |

## 5. Final gate order

0. Settled on 2026-09-13: the host condition was a stale installed binary and a
   tool catalogue bound at task creation, not a defect in the host or in this
   product. See the note in section 3.
1. Complete rows W2–W7 with the head Skill and binary, on the revision this
   matrix names. Every UI row must run in a task created after those bytes were
   installed, and must record that creation time: a task created earlier carries
   the tool catalogue of an earlier install and cannot testify about this one.
2. Complete restart, update/reinstall, uninstall, and clean reinstall on Windows;
   complete the equivalent native CLI/MCP lifecycle on WSL.
3. Either supply a production code-signing identity as protected environment
   secrets, approve the signing job and verify the timestamped signature after
   extraction, or confirm that every public surface discloses the unsigned
   runtime. This step no longer blocks the release; misdescribing it does.
4. Record the final results, commit the completed matrix, and freeze that SHA.
5. Rerun CI, the full 500k benchmark, clean Windows/WSL packaging and the
   non-publishing release workflow on that exact SHA.
6. Only then issue `AUTHORIZED TO PUBLISH`. Tagging, publishing, marketplace
   submission, and `PUBLIC RELEASE VERIFIED` remain separate later decisions.

Current verdict: **BLOCKED**.

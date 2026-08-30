# Cycle for Trae Work — Threat Model

Version 1.0.0. Scope: the local product as shipped — MCP frontend, control-plane daemon, durable state, role calls. Trae Work itself and the user's model providers are out of scope except as trust boundaries.

## Assets

| Asset | Protection |
| --- | --- |
| Original request and delivered candidate bytes | Immutable from intake; SHA-256 digest bound to every review, evidence record and verdict |
| Audit trail | Append-only hash chain with periodic Ed25519-signed checkpoints; local verification (`cycle_history_verify`) |
| Ledger signing key | Generated locally (`runtime/ledger.key`); never leaves the data directory |
| Role API keys | Referenced, never stored: `api_key_env` / `api_key_file` in `roles.json`; read at call time, redacted in every output, excluded from logs, ledger and exports. Authentication may be omitted only for loopback role endpoints; every non-loopback endpoint requires an explicit key source |
| Project repositories | All execution happens in a managed worktree; delivery writes exact approved bytes only |
| Control-plane database | SQLite WAL in the data directory; consistent backups via `trae-cycle backup` |

## Actors

- **User** — full local authority; owns confirmation for destructive operations.
- **Trae Work session (executor model)** — edits inside the authorized worktree scope; cannot approve its own work.
- **Read-only role endpoints** — four user-configured models reached over HTTPS (or localhost); receive prompts and candidate material, return structured output. Never receive Trae Work credentials.
- **Project content** — code under delivery, including project-declared verification commands. Treated as untrusted input to the pipeline.

## Attack surface and mitigations

| Surface | Threat | Mitigation |
| --- | --- | --- |
| MCP stdio | Malformed or unauthorized requests | Single local consumer (Trae Work); protocol-validated JSON-RPC; unknown tools rejected |
| Daemon IPC (named pipe / Unix socket) | Local process impersonation | Per-boot random secret with challenge-response MAC on every frame; the secret file is private to the data directory |
| Role endpoints | Key theft, response tampering, provider outage | Keys referenced not stored; rustls TLS; strict structured-output validation with binding/advisory distinction; transient (transport, 429, 5xx) vs permanent error classification; fail-closed on missing configuration |
| Verification runner | A project command reads, changes, exfiltrates or destroys data reachable by the local account | Shells, inline interpreter code and destructive/publish verbs are rejected. Only bounded version/format probes are preapproved. Every other exact command requires an expiring, single-use user consent receipt bound to workflow, candidate, gate, command digest and worktree digest; the permission is ledgered before it becomes usable. Commands run without a shell and with a reduced environment, but there is no OS sandbox |
| Candidate integrity | Late edits smuggled past review | Freeze captures exact bytes and modes; any post-freeze change invalidates evidence and reviews by digest mismatch; promotion re-verifies and writes the frozen bytes |
| Scope escalation | Executor writes outside the authorized scope | Write scopes declared in the accepted architecture plan; blind reviewers check boundaries; arbiter requires full requirement coverage |
| Audit tampering | Silent history rewriting | Hash chain makes any edit detectable; checkpoints sign the chain head; verification failure stops the workflow and requires trusted backup restore |
| History export | Secret leakage through exports | Redaction pass over role keys and secrets before any export or history query |
| Denial of resource | Runaway loops exhausting the machine | Admission control: active-workflow ceiling, leases, CPU/memory/disk reserves; pressure pauses that do not consume repair cycles |
| Repair abuse | Infinite retry storms | Maximum five repair cycles, then `blocked` pending explicit user action |

## Residual risks (accepted)

- A worktree isolates Git state, not operating-system authority. An approved verification command can read or modify any path the local account can reach and can use raw network APIs. Consent makes that authority explicit and auditable; it does not sandbox it. Users should reject commands from repositories they do not trust.
- A fully compromised local machine defeats all local guarantees (key exfiltration, ledger forgery at rest). The ledger detects tampering; it cannot prevent it.
- Role endpoint quality bounds delivery quality. The arbiter cannot detect a reviewer that lies coherently; binding verdicts bind responsibility, not truth.
- The verification chain is local. It proves consistency of the local record, not external transparency.

## Notable non-goals

No telemetry, no Cycle cloud account, no Trae Work credential access. The control plane starts commands in the managed worktree, but an approved process is not prevented by Cycle from reaching other local paths or the network.

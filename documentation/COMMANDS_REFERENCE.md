# Cycle for Trae Work — Commands Reference

Version 1.0.0. Two surfaces exist: the `/cycle` command inside Trae Work and the `cycle_*` MCP tools underneath. Users type commands; the skill and the command route to tools. The schema returned by `tools/list` is authoritative for arguments.

## Conventions

- Leading colon is accepted: `/cycle:resume` equals `/cycle resume`.
- Tools marked **job** return a `jobId` immediately; the result lands on `/cycle status` under `jobs` (`running` | `done` | `failed`, with `result` or `error`).
- Destructive operations (`cancel`, `export`, memory removal) and project-command consent require the literal `--confirm` flag plus explicit user approval in the same conversation. Tools carry the equivalent `confirm: true` argument.
- Fail-closed is the default: missing or invalid role configuration blocks every governed operation. Nothing is silently skipped or relabeled.

## `/cycle` command

| Command | Routes to | Behavior |
| --- | --- | --- |
| `run [auto\|quick\|full]` | `cycle_start` | Arms a mode (default `auto`). The next plain message becomes the immutable original request. |
| `status` | `cycle_status` | Workflow state, phase, repair budget, job results. |
| `tasks` | `cycle_tasks` | Task list with states, attempts and dependencies. |
| `evidence` | `cycle_evidence` / `cycle_status` | Registers executor evidence or summarizes registered evidence. |
| `consent <token> --confirm` | `cycle_consent` | Grants one pending command returned by `cycle_verify`; exact-command approval only, 15-minute expiry, single use. |
| `resume` | `cycle_resume` | Resumes a paused or restarted workflow from the saved phase. |
| `pause` | `cycle_pause` | Pauses at the next safe boundary. |
| `retry` | `cycle_retry` | Retries a classified transient failure; repair budget untouched. |
| `cancel --confirm` | `cycle_cancel` | Cancels; history and evidence preserved. |
| `goal ...` | `cycle_goal_*` | Goal CRUD, focus, plan revisions, links and lifecycle transitions. Ambiguity is surfaced, never guessed. |
| `memory ...` | `cycle_memory_*` | Search, explain; removal requires confirmation. |
| `history [verify]` | `cycle_history` / `cycle_history_verify` | Redacted trail; chain verification. On verification failure: stop and preserve. |
| `models` | `cycle_models` | Effective role assignments and token usage, no secrets. |
| `limits` | `cycle_limits` | Live admission policy and resource reserves. |
| `permissions` | `cycle_doctor` | Effective configuration and control-plane state. |
| `setup` | `cycle_setup` | Validates installation; reports fixes needed. |
| `doctor` | `cycle_doctor` | Read-only diagnostics with plain-language fixes. |
| `export --confirm` | `cycle_export` | Exports redacted history. |
| `help` | — | The command list. |

## MCP tools (37)

### Installation and diagnostics

| Tool | Notes |
| --- | --- |
| `cycle_setup` | Role configuration, Git availability, writable data directory, control-plane health. |
| `cycle_doctor` | Read-only diagnostics: control plane, database, ledger, roles. |
| `cycle_models` | Assignments plus per-role token usage; never secrets. |
| `cycle_limits` | Admission policy: active ceiling, lease, reserves, repair budget. |

### Workflow lifecycle

| Tool | Notes |
| --- | --- |
| `cycle_start` | `project_key`, `original_request` (verbatim), `mode`, optional `affected_paths`, `critical_downgrade_approval`. Fails closed without valid roles. |
| `cycle_status` | State, repair budget, jobs array. |
| `cycle_tasks` | Durable task list. |
| `cycle_evidence` | `session_id`, up to 1000 `files`, string `metadata` values (max 512 chars each). |
| `cycle_worktree` | Creates the isolated worktree; returns `{path, baseRevision}`. All edits belong inside `path`. |
| `cycle_index` **job** | Mandatory code-intelligence index; binds repository identity verified at promotion. |
| `cycle_submit_architecture` | Architect plan (ArchitecturePlan object) validated and accepted. |
| `cycle_execution_report` | Executor outcome: `blocked` or `plan_defect`. |
| `cycle_freeze` **job** | Plans verification and freezes exact candidate bytes; requires a clean worktree. |
| `cycle_verify` **job** | Runs the verification plan over the frozen candidate; optional managed-browser attestations. |
| `cycle_consent` | Requires explicit user approval and `confirm: true`; grants one candidate/gate/command/worktree-bound consent token. |
| `cycle_review` | Submits one binding blind review (ReviewVerdict). |
| `cycle_arbitrate` **job** | Submits the arbiter verdict (ArbiterVerdict) bound to the original request. |
| `cycle_promote` **job** | Delivers the approved exact bytes into the project directory. |
| `cycle_pause` / `cycle_resume` | Safe-boundary pause; reconcile and resume. |
| `cycle_retry` | Transient failures only; never consumes a repair cycle. |
| `cycle_cancel` | Requires `confirm: true`. |

### Role consultations

| Tool | Notes |
| --- | --- |
| `cycle_role` **job** | Operation and role must be paired: `architect_consult`/`architect`, `functional_review`/`functional_reviewer`, `security_review`/`security_reviewer`, `arbiter_readiness` and `arbiter_verdict`/`arbiter`. `executor_feasibility` fails closed — executor analysis stays in the Trae Work session. Advisory operations return advisory objects; verdict operations return binding payloads submitted unmodified through `cycle_review` / `cycle_arbitrate`. |

### Goals

| Tool | Notes |
| --- | --- |
| `cycle_goal_create` | `objective`, non-empty `success_criteria`, optional `constraints`, `non_goals`, `max_continuations` (1–255, default 5). |
| `cycle_goal_amend` | Appends an immutable amendment. |
| `cycle_goal_status` / `cycle_goal_list` | Inspect goals, plans, linked workflows. |
| `cycle_goal_focus` | Focuses one goal in the session. |
| `cycle_goal_save_plan` | New immutable plan revision. |
| `cycle_goal_link` | Links a completed-eligible workflow to a milestone. |
| `cycle_goal_control` | One transition: `start_planning`, `mark_ready`, `activate`, `pause`, `resume`, `block`, `resume_blocked`, `continue`, `request_completion`, `approve_completion`, `reject_completion`, `abort`. |

### Memory, history and export

| Tool | Notes |
| --- | --- |
| `cycle_memory_search` | `text`, optional `scope`, `confidence`, `limit` (1–1000, default 20). Local FTS5; no embeddings. |
| `cycle_memory_explain` | One entry with provenance. |
| `cycle_memory_remove` | Requires `confirm: true`. |
| `cycle_history` | Optional `after_sequence`, `limit` (1–500, default 50); redacted. |
| `cycle_history_verify` | Hash chain plus Ed25519-signed checkpoints, locally. |
| `cycle_export` | Requires `confirm: true`. |

## Boundaries

- The executor never approves its own work; approval belongs to the isolated arbiter.
- A required gate without a project-native command surfaces as `unavailable` and fails verification — skipping is not representable.
- Late file changes after freeze invalidate reviews and evidence by digest.
- Browser evidence binds to the frozen candidate digest through Trae Work `/browser_use` records registered via `cycle_evidence`.

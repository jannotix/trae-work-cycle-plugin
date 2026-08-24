# Cycle for Trae Work Design Specification

Status: Implemented — released as 1.0.0 (all phases through §27 complete; exceptions recorded in CERTIFICATION.md)
Licensor & Author: Gianluca Iannotta
License: FSL-1.1-MIT (MIT conversion after two years)
Repository: https://github.com/jannotix/trae-work-cycle-plugin
Target version: 1.0.0
Certified platforms: Windows x64 Desktop, Linux x64 Desktop (Trae Work local environment)

## 1. Problem Statement

A single-model coding session interprets a request, writes the code, reviews its own assumptions and declares completion. The result is predictable: backend endpoints without the frontend flow, migrations never applied to a real database, UI that renders but never completes the user journey, tests that pass against mocks, security controls that hold on one path only, and stupid bugs shipped as finished work.

Cycle for Trae Work removes the structural cause. It preserves the exact user request, separates planning, implementation, review and approval into isolated roles with independent model contexts, runs the project's real verification tools, and delivers only exact bytes that an independent arbiter has matched against the original request.

## 2. Product Identity

Cycle for Trae Work is a native Trae Work integration for evidence-gated software delivery. It adds one `cycle` skill surface and one local control plane. It has no dashboard, no cloud account, no standalone UI and no network service. It is not affiliated with, sponsored by or endorsed by the TRAE project.

The product name is Cycle for Trae Work. The binary and MCP server name is `trae-cycle`. The user-facing skill is `cycle-delivery`. The user-facing command is `/cycle`.

## 3. Goals

1. Deliver complete, tested, working changes or an explicit failure — never a silent partial delivery.
2. Keep the original user request immutable; the arbiter evaluates against it directly, never against the architect's summary.
3. Isolate the five roles so planner, implementer, reviewers and approver never share context or model blind spots.
4. Stay model-agnostic: every user configures their own models locally per role.
5. Be simple to configure for non-experts and precise for experts.
6. Scale to large repositories (500,000+ files) without repeated full rescans or token waste.
7. Persist and schedule 100+ workflows across projects without saturating CPU, RAM or disk.
8. Survive Trae Work updates: durable state lives outside the application directory.

## 4. Non-Goals

1. No duplication of Trae Work capabilities: planning chat (`/plan`, `/spec`), the integrated browser (`/browser_use`), workspace indexing for the host agent, sandboxing, memories, rules and built-in code review remain owned by Trae Work.
2. No unlimited autonomous loops. Repair is bounded at five cycles and goal continuation at five continuations.
3. No guarantee of defect-free software. The product makes incomplete or falsely approved work materially harder to ship, not impossible.
4. No cloud mode in v1. MCP stdio runs only in the Trae Work desktop local environment.
5. No macOS certification in v1.

## 5. System Architecture

```
[Trae Work Desktop - local environment]
  User session (executor role, model selected by the user in Trae Work)
  Skill "cycle-delivery" + command /cycle
        |
        | MCP stdio
        v
[trae-cycle binary]
  trae-cycle mcp   -> MCP stdio frontend (tool contracts, job pattern)
  trae-cycle serve -> workflowd daemon (spawned on demand, never manually)
        |
        +-- workflow-core      (workflow FSM, task DAG, risk routing, verdicts)
        +-- workflow-store     (SQLite WAL, 17+ migrations)
        +-- workflow-ledger    (SHA-256 hash chain, Ed25519 checkpoints)
        +-- workflow-memory    (provenance-backed knowledge base, FTS5)
        +-- workflow-code-intel(tree-sitter incremental graph)
        +-- workflow-roles     (read-only role calls via configured endpoints)
        +-- workflow-scheduler (resource admission, fair queues)
```

One executable, two subcommands. The MCP frontend is the only Trae Work-facing process. The daemon is per-user, shared across projects, spawned by the frontend when healthy state is absent and reclaimed when stale. The daemon never reads Trae Work credentials.

Read-only roles (architect, functional reviewer, security reviewer, arbiter) run as isolated one-shot completions against endpoints configured by the user. Only the executor runs inside the Trae Work session, because only the executor needs host tools. Contextual independence is structural: reviewers and the arbiter never see the executor's session context.

## 6. Trae Work Integration Surface

| Surface | Use |
| --- | --- |
| MCP server (stdio) | All Cycle operations exposed as tools |
| Skill `cycle-delivery` | Role discipline, evidence protocol, delivery rules |
| Command `/cycle` | Command routing and mode arming |

Timeout handling: Trae Work MCP supports `START_MCP_TIMEOUT_MS` and `RUN_MCP_TIMEOUT_MS`, but long operations (verification, delivery) exceed default client timeouts. Every tool that can run long returns a job receipt immediately; completion is observed through `cycle_status`. The skill instructs the agent never to poll in a tight loop.

## 7. Roles

| Role | Context | Capabilities | Boundary |
| --- | --- | --- | --- |
| Architect | Isolated completion via configured endpoint | Requirement matrix, risk analysis, acyclic task DAG with write scopes and per-task verification commands | Read-only; cannot implement or approve |
| Executor | Trae Work user session | Terminal, CLI, MCP servers, skills, plugins, databases, `/browser_use` under Trae Work permissions | One bounded task at a time; cannot approve its own work |
| Functional reviewer | Isolated completion | End-to-end completeness against the original request and raw evidence | Read-only; blind to the other review until finalized |
| Security reviewer | Isolated completion | Trust boundaries, dependency and supply-chain risk, architecture, resource behavior | Read-only; blind to the other review until finalized |
| Arbiter | Isolated completion | Final approve or repair verdict over request, candidate, evidence and both reviews | Cannot edit files or repair the candidate |

Blind review: the two reviewers receive the frozen candidate, the immutable request and raw evidence. Neither receives the other's verdict before finalization. The daemon rejects an approval unless exactly two approved reviews exist for a full-mode candidate.

## 8. Model Configuration

All role models are user-configured in `roles.json` inside the Cycle data directory. The file is never part of any repository or package.

```json
{
  "version": 1,
  "roles": {
    "architect": {
      "base_url": "https://api.example.com/v1",
      "model_id": "provider/model-a",
      "api_key_env": "CYCLE_ARCHITECT_KEY"
    },
    "functional_reviewer": {
      "base_url": "https://api.example.com/v1",
      "model_id": "provider/model-b",
      "api_key_file": "keys/functional.key"
    },
    "security_reviewer": { "...": "same shape" },
    "arbiter": { "...": "same shape" }
  }
}
```

Contract:

1. `api_format` is `openai_compatible` in v1. Any OpenAI-compatible endpoint works, including local aggregation gateways.
2. Credentials resolve from `api_key_env` (environment variable name) or `api_key_file` (path relative to the data directory, 0600 on Unix). Secrets are never written into `roles.json`, logs, the ledger or tool output.
3. The executor model is the model selected in Trae Work. Cycle never changes it; a mid-workflow change by the user is recorded in the audit trail at the next phase boundary.
4. Effective assignments are inspected with `cycle_models`, which reports provider, model, endpoint host and key source, never key material.
5. Missing read-only role configuration fails closed at `cycle_start` with the exact field to fix. `cycle_setup` and `cycle_doctor` validate the file and explain errors in plain language for non-experts.

## 9. Workflow Lifecycle

States: `intake`, `routing`, `quick_execution`, `architecture`, `execution`, `verification`, `independent_reviews`, `arbitration`, `delivery`, `repair`, `paused`, `blocked`, `completed`, `cancelled`.

Modes:

- `auto` (default): deterministic risk facts choose the cheapest safe route. Path-backed critical facts or two or more critical categories force full mode.
- `quick`: bounded low-risk work. Still requires governed execution, deterministic verification, frozen candidate, independent arbitration and exact delivery. No independent reviews.
- `full`: architecture, decomposition, execution, verification, two blind reviews, arbitration, repair, exact delivery.

Repair budget: five cycles. A rejected candidate returns to the executor or, for plan defects, to the architect. After the fifth rejection the workflow enters `blocked` and requires explicit user action. Infrastructure failures pause the workflow without consuming repair cycles.

Routing downgrade protection: a quick request carrying critical risk categories is promoted to full mode unless the user supplied an explicit downgrade approval.

## 10. Intake and the Immutable Request

`/cycle run` arms a mode. The next non-command user message is captured verbatim as the original request with SHA-256 digest and attachment digests. Amendments are separate immutable records. The arbiter input always includes the original request text, never a paraphrase generated by any role or by the skill.

## 11. Task Decomposition

The architect output is a validated plan: requirements with acceptance criteria, risks, assumptions, integration checks, and an acyclic task DAG. Each task carries: a bounded objective, one authorized write scope set, requirement links, dependencies and per-task verification commands. Large work is decomposed into small, independently verifiable tasks; a plan whose tasks are not individually verifiable is rejected by plan validation. Execution proceeds one task at a time inside a managed Git worktree; every completed task records changed paths, commit revision and a summary digest.

## 12. Code Intelligence

Mandatory, local and incremental. First scan inventories supported files respecting Git-compatible ignore rules, rejects symlink escapes, hashes content, parses supported languages with tree-sitter and stores an evidence-backed graph in SQLite. Later runs reprocess only affected scopes (directory partitions) for changed, renamed or deleted files. Graph edges carry provenance tags: `extracted` when explicit in source, `inferred` when resolved by analysis. Context queries are bounded by bytes, items, nodes, edges and traversal depth; truncation is reported, never hidden. Scale target: 500,000 files with incremental refresh, certified by the `codebase_500k` benchmark in the certification suite.

## 13. Verification Gates

Gate discovery is deterministic:

1. Per-task verification commands declared by the architect (mandatory).
2. Project-native commands discovered from package manifests (npm scripts, cargo, and equivalents).
3. Always-on internal gates: changed-content secret scan, candidate immutable-bytes integrity.
4. Scope-driven gates: persistence scopes require a real or disposable database check; UI scopes require a browser flow and an accessibility check; dependency scopes require vulnerability audit and license policy; packaging scopes require a production artifact and installation check.

A required gate without a project-native command surfaces as `unavailable`, which fails verification. Gates are never silently skipped or relabeled. Browser evidence is captured through Trae Work `/browser_use` by the executor and registered via `cycle_evidence`; during verification the daemon binds each browser record to the frozen candidate digest, so late file changes invalidate the evidence.

## 14. Candidate Freeze and Exact Delivery

Freeze refuses a dirty worktree. The candidate manifest records per-file digests and kinds, the exact diff, exact file bytes, executable modes, and environment, configuration and dependency digests. Review and arbitration verdicts bind to the candidate digest. Delivery verifies the source preimage against the indexed repository, reserves the delivery, promotes the exact approved bytes while preserving concurrent changes, re-verifies them and writes the completion receipt. A source that changed after freeze is an explicit delivery conflict, never a silent merge.

## 15. Review and Arbitration Criteria

Functional reviewer checklist: end-to-end user-visible behavior, frontend and backend agreement, database state, integrations, packaging, user journeys; for UI scopes, visual hierarchy, responsive behavior, accessibility and interaction completeness.

Security reviewer checklist: authentication and authorization, untrusted input, secret handling, trust boundaries, dependency and supply-chain risk, maintainability, resource behavior, production architecture; each relevant item must cite evidence.

Arbitration: the daemon rejects an approval when any mandatory gate failed, when either review is missing or not approved (full mode), when the verdict does not cover every planned requirement, or when the verdict digest does not match the frozen candidate. A rejection carries a structured repair target: execution or architecture.

## 16. Single-Role Consultations

Discussions that do not need a delivery cycle are first-class operations:

- Architect consultation: multi-turn read-only design and requirements discussion.
- Executor feasibility: read-only scope, command and environment analysis.
- Single reviewer: advisory completeness or security review of a plan or candidate.
- Arbiter readiness: advisory readiness against a goal or plan; never a delivery approval.

Consultations run through the same configured role endpoints, record session identity and model in the audit trail, and never modify project files. The user asks in plain language; the skill routes to the consultation tool. Trae Work `/plan` remains available for quick ungoverned planning; the user manual documents when each is appropriate.

## 17. Goal Mode

Durable multi-milestone outcomes with lifecycle states `draft`, `planning`, `ready`, `active`, `paused`, `blocked`, `completing`, `completed`, `aborted`. Plans are versioned and immutable per revision. `mark_ready` requires a saved architecture plan. Completion requires every linked milestone to have at least one `completed` workflow. Continuation limit: five, extendable only by explicit owner policy. Each milestone runs as a separately governed workflow linked to its goal.

## 18. Project History and Memory

History: every state transition, role action, tool invocation digest, permission decision, evidence identity, review, arbitration and delivery receipt is appended to a hash-chained ledger with periodic Ed25519-signed checkpoints. `cycle history verify` re-verifies the whole chain locally. Verification is relative to the local installation key; it is not an external transparency service.

Memory: reusable project knowledge with provenance (source events, evidence links, revision), confidence classes `verified`, `user_asserted`, `inferred`, and lifecycle `current`, `superseded`, `revoked`. Model-inferred entries can never become rules. At workflow completion the daemon captures lessons learned (what worked, what failed, repair patterns) as memory entries with provenance. Search is local SQLite FTS5; no vector database, no embeddings, no cloud.

## 19. Resource Admission and Concurrency

One per-user daemon governs all projects. Active workflow ceiling derives from logical CPUs, clamped between one and eight. Leases expire after 15 seconds and are renewed by the frontend. Admission pauses when CPU exceeds 85 percent, when the memory reserve would fall below 1 GiB, when the disk reserve would fall below 2 GiB, or when another admitted operation owns a required global resource. Projects are scheduled round-robin. Verification has priority over indexing. Recovery admits work gradually to avoid a restart stampede. The daemon persists and fairly schedules 100+ workflows across projects; simultaneous execution remains bounded by design.

## 20. Essentiality Policy

Anti-bloat gate evaluated before new code, new abstractions or new dependencies. A change proposal must provide evidence for four checks: existing implementation in the codebase, standard library coverage, native platform capability, already-installed dependency coverage. A viable alternative blocks the addition. Security, accessibility and compatibility requirements are never traded for minimality. The executor skill applies the same discipline: smallest correct solution, reuse before rewrite, never skip validation.

## 21. Cost Accounting

The daemon originates every read-only role call, so usage data is observable. Per workflow the ledger records role, model, input and output tokens and estimated cost where the endpoint reports usage. `cycle status` and `cycle history` expose per-role totals. No telemetry leaves the machine.

## 22. MCP Tool Contracts

All tools are namespaced `cycle_*`. Long-running operations follow the job pattern: immediate receipt with `job_id`, observation via `cycle_status`.

| Tool | Purpose |
| --- | --- |
| `cycle_setup` | Validate installation, models, roles.json, Git, resources |
| `cycle_doctor` | Read-only diagnostics: daemon, database, ledger, protocol, disk |
| `cycle_start` | Arm or launch a workflow from the captured request (auto/quick/full) |
| `cycle_status` | Workflow state, phase, job results, repair budget |
| `cycle_tasks` | Durable task list with ownership, dependencies, attempts, states |
| `cycle_evidence` | Register executor evidence records (command output digest, browser results) |
| `cycle_submit_architecture` | Submit the architect plan for validation and acceptance |
| `cycle_execution_report` | Report task completion, blockage or plan defect |
| `cycle_verify` | Run the verification plan over the frozen candidate (job) |
| `cycle_freeze` | Freeze the exact candidate (job) |
| `cycle_review` | Submit an independent review verdict |
| `cycle_arbitrate` | Submit the arbiter verdict (job) |
| `cycle_promote` | Deliver the approved exact bytes (job) |
| `cycle_pause` / `cycle_resume` | Pause at the next safe boundary; reconcile and resume |
| `cycle_retry` | Retry a classified transient failure |
| `cycle_cancel` | Cancel with confirmation; history and evidence preserved |
| `cycle_role` | Single-role consultation (job) |
| `cycle_goal_create` / `cycle_goal_amend` / `cycle_goal_status` / `cycle_goal_list` / `cycle_goal_focus` / `cycle_goal_save_plan` / `cycle_goal_link` / `cycle_goal_control` | Goal lifecycle |
| `cycle_memory_search` / `cycle_memory_explain` / `cycle_memory_remove` | Project memory |
| `cycle_history` / `cycle_history_verify` | Redacted audit query; chain verification |
| `cycle_models` | Effective per-role assignments without secrets |
| `cycle_limits` | Live admission policy and resource reserves |
| `cycle_export` | Redacted history export with confirmation |

Every tool result is plain structured data the skill relays verbatim. The daemon is the authority: role outputs that do not satisfy the binding rules are rejected regardless of what the skill or model claims.

## 23. User Command Surface

| Command | Purpose |
| --- | --- |
| `/cycle` | Entry point; routes to the skill |
| `/cycle run [auto\|quick\|full]` | Arm the next request |
| `/cycle status` / `/cycle tasks` / `/cycle evidence` | Progress inspection |
| `/cycle pause` / `/cycle resume` / `/cycle retry` / `/cycle cancel --confirm` | Lifecycle control |
| `/cycle goal ...` | Goal operations |
| `/cycle memory ...` | Memory operations |
| `/cycle history` / `/cycle history verify` | Audit |
| `/cycle models` / `cycle limits` / `/cycle permissions` | Configuration inspection |
| `/cycle setup` / `/cycle doctor` | Installation and diagnostics |
| `/cycle export --confirm` | Export |
| `/cycle help` | Command list owned by the runtime |

All commands are documented in the user manual because users must know them; the skill invokes them automatically when needed.

## 24. Data Directory and Durability

| Platform | Data directory |
| --- | --- |
| Windows | `%LOCALAPPDATA%\Trae Cycle` |
| Linux | `${XDG_DATA_HOME:-~/.local/share}/trae-cycle` |

Contents: `control-plane.db`, `runtime/` (IPC secret, ledger key, pid), `worktrees/`, `browser/` artifacts, `config/roles.json`, `keys/`. Nothing is written into the Trae Work installation directory. Trae Work updates cannot alter or destroy Cycle state. Plugin removal preserves project files and durable state by design.

## 25. Security Model

1. IPC: named pipe (Windows) with per-user SDDL derived from the current user SID, remote clients rejected; Unix domain socket 0600 with peer credential check. Challenge-response HMAC-SHA256 with domain separation, 32-byte nonce, five-second expiry, replay rejection.
2. Daemon trust boundary: every request validated against durable workflow ownership; project keys, identifiers and paths are bounded and sanitized; relative paths only.
3. Secrets: never in `roles.json`, never in ledger, logs, exports or tool output; key files 0600; ledger export applies redaction.
4. Prompt injection: repository content, tool output, model output are untrusted data; they cannot override user intent, role boundaries or permission policy.
5. Full threat model maintained as a separate document.

## 26. Error Taxonomy and Recovery

| Condition | Behavior |
| --- | --- |
| `cpu_pressure`, `memory_pressure`, `disk_pressure` | Admission pauses; retry does not consume a repair cycle |
| `concurrency_limit`, `fair_queue` | Wait for normal admission |
| `blocked` after five rejections | Explicit user action required |
| Mandatory gate failed | Fix the real failure; skipping is not representable |
| Candidate changed after freeze | Reviews invalidated; new freeze required |
| Delivery conflict | Both states preserved; reconcile through a new cycle |
| Role endpoint failure | Classified transient or configuration error; transient retries do not consume repair cycles |
| History verification failure | Stop, preserve data, restore trusted backup |

## 27. Implementation Plan (Bounded Tasks)

| Phase | Tasks | Exit criteria |
| --- | --- | --- |
| 1. Workspace | Copy Rust crates; rename product identity, domains, data paths; workspace lint gates | `cargo fmt --check`, `clippy -D warnings`, `cargo test` green |
| 2. MCP frontend | `trae-cycle mcp` stdio server; tool schemas; job pattern; daemon lifecycle | E2E MCP client harness green over real daemon |
| 3. Role calls | `workflow-roles` crate; roles.json loader; structured outputs for plan, reviews, verdicts; usage capture | Deterministic fake-endpoint tests green |
| 4. Skill and command | `cycle-delivery` SKILL.md; `/cycle` command; evidence protocol; consultation flows | Manual protocol walkthrough in Trae Work |
| 5. Certification | Install/uninstall, full autonomous cycles quick and full, repair path, all commands, 500k scale, concurrent projects | Certification report green on Windows x64 and Linux x64 |
| 6. Release | Packaging allowlist, CI provenance verification, SHA256SUMS, SBOM, license gate | Verified public release |

Each phase lands as small vertically complete tasks with a failing test or deterministic check written before non-trivial logic.

## 28. Testing Strategy

1. Unit: crates keep existing suites; new code follows test-first discipline.
2. Contract: MCP tool schemas validated against fixtures; daemon responses validated in both directions.
3. E2E: harness drives the MCP frontend against a real daemon with a seeded Git repository, deterministic fake role endpoints and scripted verification commands.
4. Failure injection: endpoint outages, malformed role output, dirty worktrees, mid-freeze file changes, delivery conflicts, resource pressure.
5. Scale: `codebase_500k` benchmark; concurrent multi-project admission test.
6. Certification: scripted local suite executed on the owner's Windows machine and a Linux x64 runner before any publication.

## 29. Packaging and Release

Production artifacts are assembled from explicit allowlists and exclude documentation, tests, fixtures, debug output, logs, coverage and development configuration. Release pipeline verifies artifact provenance (manifest revision equals source revision), publishes the plugin package and skill archive with SHA256SUMS, generates a CycloneDX SBOM and enforces a permissive SPDX license allowlist compatible with FSL-1.1-MIT. Publication happens only after the Phase 5 certification gate is green.

## 30. Documentation Set

1. `SPECIFICATION.md` (this document).
2. `USER_MANUAL.md`: installation, model configuration for non-experts, modes, commands, goals, recovery, removal.
3. `COMMANDS_REFERENCE.md`: every command and tool with behavior and boundaries.
4. `THREAT_MODEL.md`: assets, actors, attack surface, mitigations.
5. `CHANGELOG.md`, `ROADMAP.md`.

## 31. Acceptance Criteria (v1)

1. A full-mode workflow completes autonomously from an exact user request to exact delivered bytes with two blind reviews and an arbiter approval bound to the original request.
2. A quick-mode workflow completes with verification and arbitration, without reviews.
3. A rejected candidate enters bounded repair; five rejections block; infrastructure failures do not consume cycles.
4. The arbiter never approves: a candidate with failed mandatory gates, missing reviews, uncovered requirements, or digest mismatch. These cases are regression-tested.
5. Install and uninstall on the owner's Windows machine preserve project files and durable state; `/cycle doctor` and `/cycle history verify` pass after reinstall.
6. All documented commands execute correctly in the certification suite.
7. 500,000-file repository refresh is incremental after first index; concurrent multi-project workflows respect admission limits.
8. No secrets appear in roles.json, logs, ledger or exports.
9. Trae Work application updates leave Cycle state intact.
10. Production archives contain production files only.

## 32. Open Items

None blocking. Phase 1 begins on owner approval of this specification.

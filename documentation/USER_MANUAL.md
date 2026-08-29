# Cycle for Trae Work — User Manual

Version 1.0.0. The control plane, state and verification runner are local; Cycle has no account, cloud service or telemetry. User-configured role endpoints may be cloud services and receive the bounded candidate/request material required for their role.

## 1. Requirements

- Trae Work Desktop on Windows 10/11 x64, or WSL2 Ubuntu x64 for the CLI/MCP runtime lane.
- The `trae-cycle` binary from the release archive for your platform.
- Git available on `PATH` (worktrees and delivery use the project's real Git repository).
- Four model endpoints for the read-only roles (any OpenAI-compatible server: a cloud provider or a local runtime such as Ollama or LM Studio).

The executor role is the model you already selected in Trae Work. Cycle never changes it and never reads Trae Work provider keys.

## 2. Installation

1. Unpack the release archive and place the executable in a permanent location. The Windows MCP `command` path must not contain spaces; use `%LOCALAPPDATA%\TraeCycle\bin\trae-cycle.exe`. In WSL use `~/.local/share/trae-cycle/bin/trae-cycle` and retain mode `0755`.
2. In Trae Work settings, add an MCP server (local environment) with the contents of `plugin/install/mcp.example.json`, replacing the command path with your binary location and the data directory if you do not want the default.
3. Upload `cycle-delivery-skill-<version>.zip` in Trae Work's Skills marketplace. Its `SKILL.md` is at the archive root. A manual alternative is to unpack it under `%USERPROFILE%\.trae-cn\skills\cycle-delivery\`.
4. Create the `cycle` command in Trae Work settings from `plugin/command/cycle.md`.
5. Configure the role models (next section).
6. Run `/cycle setup`, then `/cycle doctor`. Both must report a healthy installation before the first cycle.

The certified v1 lanes are Windows x64 and WSL2 Ubuntu x64. macOS is **compatible but untested**; no macOS result substitutes for either required lane.

### Verification command consent

Internal integrity checks and a narrow set of version/format probes are preapproved. Build, test, package-manager and repository scripts execute with the local user's operating-system privileges and are not sandboxed by Cycle. `cycle_verify` therefore returns the exact command vector and a consent token before each nonpreapproved command. Review it, then approve only that command with `/cycle consent <token> --confirm`. The receipt expires after 15 minutes, is single-use, and is bound to the workflow, frozen candidate, gate, command and worktree. Reject commands from repositories you do not trust.

## 3. Model configuration

Create `%LOCALAPPDATA%\Trae Cycle\config\roles.json` (or the equivalent file inside your chosen data directory). All four roles are mandatory; any missing or invalid entry fails every governed operation until fixed — this is deliberate.

```json
{
  "version": 1,
  "roles": {
    "architect": {
      "base_url": "https://api.openai.com/v1",
      "model_id": "gpt-4.1",
      "api_key_env": "CYCLE_ARCHITECT_KEY"
    },
    "functional_reviewer": {
      "base_url": "http://localhost:11434/v1",
      "model_id": "qwen2.5:14b"
    },
    "security_reviewer": {
      "base_url": "http://localhost:11434/v1",
      "model_id": "qwen2.5:14b"
    },
    "arbiter": {
      "base_url": "https://openrouter.ai/api/v1",
      "model_id": "openai/gpt-4.1",
      "api_key_env": "CYCLE_ARBITER_KEY"
    }
  }
}
```

- `base_url` is the server root; Cycle appends `/chat/completions`.
- Authentication is optional (local servers need none). With authentication, prefer `api_key_env` (name of an environment variable) over `api_key_file` (path to a file containing the key). Keys are read at call time, never stored and never logged.
- Roles can share an endpoint or use four different providers. `/cycle models` shows the effective assignment and token usage without ever printing a key.
- Recommended: the arbiter on the strongest model you can afford. It compares the original request against the delivered candidate; weak arbiters produce weak deliveries.

## 4. Modes

Arm a mode, then write the request. The request text becomes immutable: it is hashed, and every later phase is bound to that hash.

- `/cycle run auto` — the default. Cycle classifies the request by risk and routed scopes; small work stays quick, risky work takes the full path.
- `/cycle run quick` — architect plan, executor work in an isolated worktree, real verification, arbiter approval. No blind reviews.
- `/cycle run full` — adds two binding blind reviews (functional, security and architecture) before arbitration. Reviewers never see each other's verdict; the arbiter sees everything starting from your original request.

After the armed confirmation, your next plain message is the original request. Keep it exact and complete: it is the contract the delivery is measured against.

## 5. Everyday commands

```text
/cycle status      state, phase, repair budget, background jobs
/cycle tasks       task list with dependencies
/cycle resume      resume a paused workflow (/cycle:resume after a restart)
/cycle retry       retry a transient failure (does not consume a repair cycle)
/cycle pause       pause at the next safe boundary
/cycle cancel --confirm   cancel, preserving history and evidence
/cycle export --confirm   export redacted history
/cycle history verify     verify the local audit chain
```

`/cycle help` lists everything.

## 6. Single-role consultations

Outside a governed cycle you can consult one role directly: `/cycle architect`, `/cycle reviewer`, `/cycle security`, `/cycle arbiter` followed by the question. Advisory answers stay advisory; they never modify durable state.

## 7. Goals

For multi-milestone work, `/cycle goal create` records an objective with success criteria, then planning, milestones and linked workflows:

```text
/cycle goal create ...
/cycle goal status
/cycle goal control mark_ready
/cycle goal control approve_completion
```

Plans are versioned and immutable per revision; milestones complete only with at least one completed linked workflow.

## 8. Memory and history

- `/cycle memory search <text>` — reusable project knowledge with provenance and confidence. Nothing model-inferred ever becomes a rule.
- `/cycle history` — the redacted audit trail of everything the control plane did.
- `/cycle history verify` — re-verifies the hash chain and signed checkpoints. On failure: stop, preserve data, restore from backup.

## 9. Recovery

| Situation | What happens |
| --- | --- |
| Role endpoint down or rate-limited | Failure classified transient; `/cycle retry` re-attempts without consuming a repair cycle |
| Verification fails | The candidate returns to execution with the failing gate as the repair target |
| Arbiter rejects | Execution resumes with a structured repair target; five rejections block pending explicit user action |
| Files changed after freeze | Reviews invalidated; a new freeze is required |
| Trae Work updated or restarted | Durable state is untouched; `/cycle:resume` continues the workflow |
| Machine lost | Restore the data directory from backup; verify with `/cycle history verify` |

Backups: `trae-cycle backup --data-dir <dir> --to <file>` produces a consistent copy of the control-plane database.

## 10. Removal

1. Remove the `trae-cycle` MCP server entry and the `cycle` command from Trae Work settings.
2. Stop any running daemon (`Get-Process trae-cycle | Stop-Process -Force` on Windows).
3. Delete the data directory (default `%LOCALAPPDATA%\Trae Cycle`).
4. Delete the `cycle-delivery` skill directory.

Your project files and Git repositories are never touched by removal.

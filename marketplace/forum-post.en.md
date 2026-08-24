# [Skill Share] Cycle for Trae Work — evidence-gated delivery: five isolated roles, blind reviews, arbiter approval, exact-byte promotion

## What it is

Cycle for Trae Work is a locally-run delivery governance plugin. It does not replace TRAE planning, browsing or review — it adds an evidence gate between "the agent says it's done" and "it is actually delivered". Open source (FSL-1.1-MIT, converts to MIT after two years). Fully local: no cloud, no account, no telemetry.

## The problem it solves

In a single chat the AI interprets the request, writes the code, and grades its own homework. Unfinished layers hide: the backend without the UI flow, the migration never applied, tests passing against mocks. Cycle freezes the original request as an immutable contract, separates execution from approval, and records inspectable evidence before anything ships.

Five isolated roles:

1. **Architect** — turns the original request into a task DAG with acceptance criteria;
2. **Executor** — works one authorized scope at a time inside a managed worktree (this is the TRAE session itself; its model is unchanged);
3. **Functional reviewer** — blind review: frozen candidate, original request, raw evidence only;
4. **Security reviewer** — second blind review; neither reviewer sees the other;
5. **Arbiter** — final verdict over the user's **original request**, the candidate digest, the evidence and both reviews. The executor can never approve its own work.

Key mechanics: immutable request (SHA-256 bound to every phase), candidate freeze (exact bytes/diff/modes; any post-freeze change invalidates evidence), real verification (the project's own commands, not the model's summary), exact-byte delivery via `cycle_promote`, bounded repair (5 cycles, then blocked; infrastructure failures never consume one), and a hash-chained audit ledger verifiable with `/cycle history verify`.

Every role model is user-configured: four read-only roles on any OpenAI-compatible endpoint (cloud or local Ollama/LM Studio). Cycle never reads TRAE credentials.

## Install (5 steps, ~5 minutes)

1. Download the platform archive from [GitHub Releases](https://github.com/jannotix/trae-work-cycle-plugin/releases) and unpack `trae-cycle`;
2. Register the MCP server from `plugin/install/mcp.example.json` (local environment);
3. Unpack the `cycle-delivery` skill into the global skills directory;
4. Create the `cycle` command from `plugin/command/cycle.md`;
5. Configure the four role models in `config/roles.json`, then `/cycle setup` and `/cycle doctor`.

## Example

```text
/cycle run quick
add a rate limit to the login endpoint
```

Flow: architecture plan → execution in a managed worktree → real verification → candidate freeze → arbitration → exact-byte delivery. Full mode adds two blind reviews on top. Single-role consultations work too: `/cycle architect is splitting this service worth it?`

## Certified numbers

- Windows x64 certification: autonomous quick/full cycles, repair path, all 36 tools, concurrent projects — green;
- 500k-file repository: 0 parse errors, 2.17M nodes / 1.7M edges, incremental refresh 35 s, peak memory 308 MB;
- CI on Windows and Linux: fmt + clippy `-D warnings` + 279 tests green;
- Releases ship SHA256SUMS, CycloneDX SBOM, provenance manifest and build attestations.

## Links

- Repository and docs: https://github.com/jannotix/trae-work-cycle-plugin

## Closing thought

The bottleneck of AI coding moved from "writes fast" to "verifies honestly". Cycle adds the oldest rule in software engineering: whoever implements cannot approve. Feedback and issues welcome.

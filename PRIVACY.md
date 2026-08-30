# Privacy and Data Flow

Cycle for Trae Work does not provide a Cycle cloud account, advertising SDK, or
product telemetry service. Its control plane and durable state run locally, but
the product is not necessarily offline: the user chooses the Trae Work model,
the four role endpoints, project commands, browser flows, and repositories that
may communicate over a network.

## Data stored locally

Cycle may store the following under its configured data directory:

- workflow requests, states, task plans, evidence metadata, reviews, verdicts,
  delivery receipts, and a hash-chained audit ledger;
- SQLite databases and signed ledger checkpoints;
- managed Git worktrees, code-intelligence indexes, and bounded browser evidence;
- role endpoint configuration and references to API-key environment variables or
  files; key material itself is not written to Cycle's database or ledger.

Project files remain in the selected repositories and managed worktrees. Cycle
does not write into the Trae Work installation directory.

## Data sent outside the machine

- The executor uses the model selected in Trae Work. Trae Work's own processing,
  account, synchronization, and privacy behavior are governed by Trae Work, not
  by Cycle.
- Architect, reviewer, and arbiter calls go to the endpoints configured in
  `config/roles.json`. Depending on the phase, they may receive the original
  request, relevant project context, candidate digest or content, raw evidence,
  review findings, and proposed verdict data.
- Project commands and browser checks may access network services with the local
  user's privileges. User consent authorizes an exact command; it does not add
  an operating-system sandbox or reduce that command's privileges.
- GitHub is contacted only through user-initiated repository, update, download,
  attestation, or release operations.

Cycle does not sell data or add an independent analytics stream. Users must
review the privacy and retention terms of Trae Work, every configured model
provider, repository host, browser destination, and command-line service.

## Secrets

Use `api_key_env` or `api_key_file` for non-loopback role endpoints. Keys are
read at call time and redacted from Cycle output. Never place credentials in a
repository, `roles.json`, evidence attachment, prompt, screenshot, or support
request. Authentication may be omitted only for loopback endpoints.

## Retention and deletion

Removing the Skill, Command, MCP registration, or executable does not delete
project files or the Cycle data directory. This preserves recovery evidence by
design. To remove durable Cycle data, first export any required redacted audit
record, stop the MCP server, verify the exact configured data-directory path,
and delete that directory manually. Deletion is irreversible unless a backup
exists.

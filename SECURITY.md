# Security Policy

## Supported versions

Security fixes are provided for the latest published `1.x` release only. Older
releases become unsupported when a newer `1.x` release is published. A release
is supported only on the platform lanes named in its manifest: Windows 10/11
x64 with Trae Work Desktop and WSL2 Ubuntu x64. macOS is **compatible but
untested** and has no v1 support or certification claim.

## Reporting a vulnerability

Use GitHub's private vulnerability report form:

https://github.com/jannotix/trae-work-cycle-plugin/security/advisories/new

Do not include exploit details, credentials, private source code, role prompts,
or personal data in a public issue. If the private report form is unavailable,
open a public issue containing only a request for a private security contact.

Include the affected Cycle version, Trae Work version, platform, reproduction
steps, expected impact, and whether the issue crosses a trust boundary. Remove
all secrets and personal paths from logs before attaching them.

## Security boundaries

- Cycle's control plane, database, ledger, worktrees, and configuration are
  local to the user's machine. Configured role endpoints are separate trust
  domains and receive request, candidate, evidence, review, or arbitration
  material needed for their role.
- Worktree isolation is not an operating-system sandbox. Project commands run
  with the local user's privileges. Only narrow version and format probes are
  preapproved; every other exact command needs an expiring, single-use consent
  receipt bound to the workflow, candidate, gate, invocation, and worktree.
- Authentication may be omitted only for loopback role endpoints. Non-loopback
  endpoints require `api_key_env` or `api_key_file`; keys are resolved at call
  time and are excluded from logs, the ledger, and exports.
- Release assets are accepted only when the source revision, version, manifest,
  checksums, provenance attestations, archive inventory, and required Windows
  Authenticode signature all verify.

The detailed trust model and limitations are in
[`documentation/THREAT_MODEL.md`](documentation/THREAT_MODEL.md).

## Disclosure process

Reports are triaged on a best-effort basis; no response-time SLA is offered.
Confirmed issues remain private until a fix and release guidance are available.
Publishing, tagging, or moving an existing release is never used as a shortcut:
a fix receives a new version and must pass the full release gates.

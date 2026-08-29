# Cycle for Trae Work

<p align="center">
  <img src="marketplace/logo.svg" width="160" alt="Cycle logo">
</p>

Cycle for Trae Work is a native Trae Work integration for evidence-gated software delivery. It coordinates five isolated roles without replacing Trae Work planning, browsing or review capabilities.

## What it is

A local Trae Work integration that coordinates five isolated roles:

1. **Architect** — turns the original request into a bounded, verifiable task plan.
2. **Executor** — implements one authorized scope at a time inside the Trae Work session.
3. **Functional reviewer** — checks completeness and user-visible behavior.
4. **Security and architecture reviewer** — independently checks trust boundaries and architecture.
5. **Arbiter** — approves or rejects by comparing the original user request, the exact candidate, raw evidence and both reviews. The executor cannot approve its own work.

There is no separate dashboard, cloud account or extra UI. Everything runs inside Trae Work through a local control plane.

## What it solves

A single coding session can interpret a request, write the code, review its own assumptions and declare the job done. That hides unfinished layers: a backend without the UI flow, a migration never applied, tests that pass against mocks, or a security control that exists on one path only.

Cycle for Trae Work keeps the original request immutable, separates implementation from approval, and records inspectable evidence before delivery.

## Why use it

- Small work stays cheap (`auto` / `quick`). Risky work takes the full independent cycle.
- Real project tools and Git freeze/delivery, not a summary of what an agent claims it did.
- Every role can run a different model configured locally by each user; the integration is model-agnostic.
- Durable state lives outside the Trae Work installation, so application updates do not wipe workflows.
- Windows 10/11 x64 with Trae Work Desktop and WSL2 Ubuntu x64 are the certified v1 lanes.
- macOS is **compatible but untested** and carries no v1 support or certification claim.

## Install

Requirements: Trae Work Desktop on Windows x64 for the host integration, or WSL2 Ubuntu x64 for the native CLI/MCP runtime lane. No Rust toolchain, service account or Trae Work credential is involved.

1. Download the platform archive from [Releases](https://github.com/jannotix/trae-work-cycle-plugin/releases) (`trae-cycle-windows-x64.zip` or `trae-cycle-wsl-x64.tar.gz`). On Windows, unpack `trae-cycle.exe` to a permanent path with no spaces, for example `%LOCALAPPDATA%\TraeCycle\bin\`. In WSL, unpack `trae-cycle` to `~/.local/share/trae-cycle/bin/` and keep it executable.
2. Register the MCP server in Trae Work settings (local environment) using `plugin/install/mcp.example.json` as the template, with your unpacked paths.
3. Upload `cycle-delivery-skill-<version>.zip` from Trae Work's Skills marketplace, or unpack it into `%USERPROFILE%\.trae-cn\skills\cycle-delivery\`. The upload archive carries `SKILL.md` at its root as required by Trae Work.
4. Create the `cycle` command in Trae Work settings from `plugin/command/cycle.md`.
5. Configure the four read-only role models in `%LOCALAPPDATA%\Trae Cycle\config\roles.json` (any OpenAI-compatible endpoint; see the user manual). Cycle never reads Trae Work provider keys.
6. Verify:

```text
/cycle setup
/cycle doctor
```

## Usage

```text
/cycle run auto
/cycle status
/cycle help
```

The user manual and command reference cover modes, role consultations, goals, recovery and removal.

## Documentation

| Document | Contents |
| --- | --- |
| [User manual](documentation/USER_MANUAL.md) | Installation, model configuration, modes, goals, recovery, removal |
| [Commands reference](documentation/COMMANDS_REFERENCE.md) | Every command and tool with behavior and boundaries |
| [Design specification](documentation/SPECIFICATION.md) | Architecture, workflow lifecycle, gates, acceptance criteria |
| [Certification report](documentation/CERTIFICATION.md) | Windows x64 certification runs and 500k-file benchmark |
| [Threat model](documentation/THREAT_MODEL.md) | Assets, actors, attack surface, mitigations |
| [Roadmap](documentation/ROADMAP.md) | Direction after 1.0 |

## License

FSL-1.1-MIT. See `LICENSE` and `NOTICE`. Cycle for Trae Work is an independent integration. It is not affiliated with, sponsored by or endorsed by the TRAE project.

Repository: https://github.com/jannotix/trae-work-cycle-plugin

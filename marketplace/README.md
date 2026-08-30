# Marketplace Kit

Distribution assets for Cycle for Trae Work. Trae Work exposes built-in [Marketplace and Skills](https://solo.trae.ai/skills/) surfaces, but no public third-party publisher schema or submission endpoint was documented by Trae Work as of 2026-08-30. The official catalog therefore remains curated; this kit provides the exact listing metadata and install recipe for a manual review submission.

## Files

| File | Purpose |
| --- | --- |
| `logo.svg`, `logo-400.png` | Product logo (vector source and the 400×400 PNG marketplace norm) |
| `manifest.json` | Marketplace listing draft: identity, keywords, MCP/skill/command components |
| `install-recipe.md` | Exact Windows and WSL installation, verification, update and removal contract |
| `SUBMISSION_CHECKLIST.md` | Fail-closed preflight for the exact marketplace candidate |
| `forum-post.zh-CN.md`, `forum-post.en.md` | Ready-to-past posts for the official TRAE forum |
| `business-pitch.md` | One-page pitch for the TRAE Work marketplace team (ByteDance) |

## Executed submissions

- `trae-community/trae-skills`: [pull request #24](https://github.com/trae-community/trae-skills/pull/24) remains open and blocked as of 2026-08-30. It must be refreshed to the final release bytes only after public release verification; its current head is not a production receipt.

## Manual channels (need your account)

1. **Official forum (forum.trae.cn)** — highest-visibility channel watched by the TRAE team. Register, then post `forum-post.zh-CN.md` (Chinese is the primary language there; attach `forum-post.en.md` at the end if you like). Attach the logo and, if possible, a screen recording of a `/cycle run quick` delivery.
2. **agentskill.sh** — go to `https://agentskill.sh/submit`, paste `https://github.com/jannotix/trae-work-cycle-plugin`. Note: it prefers a `SKILL.md` at the repository root or under `.cursor/skills/<name>/`; ours lives at `plugin/skill/cycle-delivery/SKILL.md`. If the analyzer does not find it, decide whether to add a root-level copy (duplicate maintenance) before submitting.
3. **skills.sh** — no form; listing is triggered by the first install: `npx skills add jannotix/trae-work-cycle-plugin`. Be aware this installs the skill into every compatible agent directory on the machine that runs it.
4. **LobeHub MCP marketplace** — `https://lobehub.com/publish-mcp` (GitHub login required), submit the MCP server with the repository URL.
5. **ByteDance / Trae Work marketplace team** — submit `business-pitch.md`, the manifest, install recipe, policies, logo, and exact release links through a verified official business contact. External submission is representational communication and requires owner confirmation at action time.

## Maintenance

When a new version ships, bump `version` in `marketplace/manifest.json` and in `production/Cargo.toml` together; regenerate `logo-400.png` from `logo.svg` only if the logo changes.

# Support, Updates, and Rollback

## Supported platform contract

The v1 certified lanes are Windows 10/11 x64 with Trae Work Desktop and WSL2
Ubuntu x64 for the native CLI/MCP runtime. The exact Trae Work version certified
for a release is recorded in its certification report. macOS is **compatible but
untested** and carries no v1 support SLA. Native Linux desktop outside WSL is not
a v1 Trae Work certification target.

Only the latest published `1.x` release is supported. Support is best effort;
there is no guaranteed response or resolution time.

## Asking for help

Use GitHub Issues for non-sensitive defects and installation questions:

https://github.com/jannotix/trae-work-cycle-plugin/issues

Include the Cycle version, Trae Work version, platform, installation method,
exact failing command or UI step, and sanitized output. Never attach API keys,
role prompts, private source, personal paths, or an unredacted Cycle export.
Security issues must follow [`SECURITY.md`](SECURITY.md).

## Safe update procedure

1. Finish or pause active workflows at a safe boundary.
2. Back up the configured Cycle data directory with `trae-cycle backup` and
   preserve its hash outside that directory.
3. Download the new immutable release assets. Verify `SHA256SUMS.txt`,
   `MANIFEST.json`, GitHub provenance attestations, and the Authenticode signature
   of the extracted Windows executable before running it.
4. Stop the local MCP server. Replace the executable only with the verified
   platform asset; then update the Skill and Command from the same release.
5. Start Trae Work and run `/cycle doctor` followed by `/cycle history verify`.
   Resume work only when both succeed.

Trae Work application updates must not alter Cycle's data directory. After a
Trae Work update, confirm the MCP entry, Skill, and Command remain enabled, then
repeat the doctor and history checks.

## Rollback

Do not point an older Cycle binary at a data directory already migrated by a
newer release. Stop the MCP server and restore both the previously verified
binary/Skill/Command and the matching pre-update data-directory backup. Verify
their recorded hashes, run `/cycle doctor`, and run `/cycle history verify`
before resuming. If no matching backup exists, keep the newer version installed
and request support; guessing around a schema mismatch can destroy evidence.

Release tags and assets are immutable. `v1.0.0` is never moved or reused; every
update or rollback correction receives a new version.

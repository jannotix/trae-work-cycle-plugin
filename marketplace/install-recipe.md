# Marketplace Install Recipe

Cycle for Trae Work uses Trae Work's native local MCP, Skill, and Command
surfaces. The official catalog does not currently expose a public third-party
publisher schema or one-click binary installer, so the production listing must
use this explicit recipe until Trae Work supplies an accepted bundle contract.

## Windows 10/11 x64 with Trae Work Desktop

1. Download `trae-cycle-windows-x64.zip`,
   `cycle-delivery-skill-<version>.zip`, `MANIFEST.json`, and
   `SHA256SUMS.txt` from the same immutable GitHub Release.
2. Verify the manifest, checksums, GitHub provenance attestations, and the
   Authenticode signature and trusted timestamp of `trae-cycle.exe`.
3. Extract the runtime archive. It must contain exactly `trae-cycle.exe`,
   `README.md`, `LICENSE`, `NOTICE`, and `THIRD-PARTY-NOTICES.md`.
4. Copy `trae-cycle.exe` to `%LOCALAPPDATA%\TraeCycle\bin\`. The executable
   path contains no spaces. Durable data may remain in `%LOCALAPPDATA%\Trae Cycle`.
5. In Trae Work, open Settings → MCP → Local → Add → Configure Manually and
   register the executable with arguments `mcp`, `--data-dir`, and the absolute
   data-directory path. Never embed credentials in the MCP JSON.
6. In Marketplace → Skills → Upload Skill, upload the release Skill ZIP. The
   archive has `SKILL.md`, `LICENSE`, and `NOTICE` at its root.
7. In Settings → Commands → Local → Create, create `cycle` using
   `plugin/command/cycle.md` from the matching plugin package.
8. Configure the four roles in `<data-dir>\config\roles.json`. Authentication
   may be omitted only for loopback endpoints; every other endpoint requires an
   environment-variable or file key reference.
9. Run `/cycle setup`, `/cycle doctor`, and `/cycle history verify` before the
   first delivery.

## WSL2 Ubuntu x64

1. Download and verify `trae-cycle-wsl-x64.tar.gz` from the same release.
2. Extract it on the Linux filesystem, not under `/mnt/c`. The archive must
   contain the same license material and `trae-cycle` with mode `0755`.
3. Install the executable under `~/.local/share/trae-cycle/bin/` and use an
   absolute Linux data-directory path.
4. Run the extracted MCP smoke and the CLI/control-plane certification matrix.
   WSL is a native CLI/MCP lane; it does not substitute for the Windows Trae Work
   UI receipt.

## Platform label

Windows x64 and WSL2 Ubuntu x64 are the certified v1 lanes. macOS is
**compatible but untested** and has no v1 support SLA. Native Linux desktop
outside WSL is not a v1 Trae Work certification target.

## Removal

Disable and remove the MCP entry, Skill, and Command, then remove the installed
executable. Durable Cycle data and project files are intentionally preserved.
Delete the data directory only after verifying its exact path and retaining any
required backup or redacted audit export.

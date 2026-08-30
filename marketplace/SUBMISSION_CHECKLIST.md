# Trae Work Marketplace Submission Checklist

Submission is permitted only for the exact candidate that has reached
`AUTHORIZED TO PUBLISH`. A marketplace review request that changes source,
packaging, permissions, data-flow claims, or listing assets creates a new
candidate and invalidates prior certification receipts.

- [ ] Listing name, version, source tag, and manifest revision are coherent.
- [ ] Windows 10/11 x64 and WSL2 Ubuntu x64 are the only certified v1 labels.
- [ ] macOS wording is exactly `compatible but untested`.
- [ ] Logo is the tracked 400×400 PNG or its tracked SVG source.
- [ ] Description does not claim fully local or sandboxed execution.
- [ ] Filesystem, process, Git, browser, and network permissions match
      `marketplace/manifest.json` and the threat model.
- [ ] Privacy, security, support, update, rollback, and license links resolve at
      the immutable public release tag.
- [ ] Skill ZIP has root-level `SKILL.md`, `LICENSE`, and `NOTICE` and passes an
      extracted upload smoke in the pinned Trae Work version.
- [ ] Windows executable is Authenticode-signed and timestamped; both runtime
      archives include complete license and third-party attribution material.
- [ ] Every release asset matches `MANIFEST.json`, `SHA256SUMS.txt`, and its
      GitHub provenance attestation.
- [ ] Windows UI and WSL clean-install certification receipts bind the exact
      release SHA and bytes.
- [ ] The owner has approved the external submission action and destination.
- [ ] Submission response, reviewer identity, date, and candidate SHA are
      recorded without credentials or private account data.

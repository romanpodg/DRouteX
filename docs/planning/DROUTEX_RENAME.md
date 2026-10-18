# DRouteX project rename

Owner authorization: direct chat instruction on 2026-10-18 to rename the repository,
project contents and local directory to DRouteX. The owner separately accepted
incompatible old cryptographic records/signatures and requested replacement where
possible. This is a separate project maintenance change, not the next roadmap
implementation task and not a resolution of the open P1-05 security gate.

## Scope and compatibility

- GitHub repository: `romanpodg/DRouteX`; local directory: `Desktop/DRouteX`.
- Rust packages/imports use `droutex-*` / `droutex_*`; environment variables use
  `DROUTEX_*`. Documentation, specification filenames, scripts and links follow
  the new name. EN/RU specification content changes only by product-name replacement;
  their new SHA-256 values are pinned in the existing preservation checks.
- Storage filenames are `droutex.db` (including WAL/SHM) and `droutex.lock`.
  The encrypted-record marker is `DXSEC\0\x00\x01`; its authenticated domain is
  `DRouteX secret record\0`. Previous ciphertext is intentionally incompatible.
  No deployed service or operational database was migrated; synthetic records
  are recreated by the existing tests. Key material is not renamed, exposed or
  re-provisioned by this change.
- The local, unpublished P1-05 prototype also uses `DXSNAP\0\x01` and
  `DRouteX snapshot v1\0`. Its public RFC8032 golden fixture was regenerated
  independently with OpenSSL. The prototype remains local and PR #9 remains
  draft; the weak-trust regression still blocks production signature acceptance.
- Git history and historical external review/CI records remain preserved. New
  names and links do not turn historical evidence into fresh DRouteX validation.
  Existing external integration results must be verified after publication.

Read-only GitHub verification on 2026-10-18 confirmed
[PR #10](https://github.com/romanpodg/DRouteX/pull/10) merged at
`9f1aa92b50bd2efd745a96104b0d3c9694764d38`. The current local workspace is
`C:\Users\roman\Desktop\DRouteX` and its origin is `romanpodg/DRouteX`.
This verifies rename publication, not fresh runtime or P1-05 acceptance.

## Validation

Local and published checks are recorded in the rename pull request. Required
validation includes locked Rust checks/tests, planning/link/hash validation,
qualification packages, unchanged vendored-source fingerprints and real Linux CI.
Local backup/diffs/logs stay under ignored `.setup/rebrand/`.

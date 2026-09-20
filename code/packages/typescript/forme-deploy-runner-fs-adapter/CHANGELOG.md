# Changelog — @coding-adventures/forme-deploy-runner-fs-adapter

## 0.1.0 — 2026-09-20

Initial FM-B045 release.

### Added

- Complete-tree filesystem publication from strictly validated deploy
  manifests and core-verified content snapshots.
- Private sibling staging, an exclusive per-root lock, a same-parent backup,
  explicit commit/finalize/rollback states, and one-shot publication.
- Write-free exact-tree detection, stale-file and stale-directory pruning,
  cooperative cancellation, retry-safe phased rollback, and deterministic
  cleanup that preserves primary failure diagnostics.
- Streaming existing-tree scans bounded by entry count, nesting depth, and
  portable-path metadata bytes.
- Fail-closed nested-entry race handling and final cancellation checks before a
  prepared transaction can escape to its caller.

### Security

- Reject linked roots, linked path components, non-regular entries, and files
  with multiple hard links before publication.
- Create staging files exclusively, verify canonical containment and exact
  size/digest after writing, and revalidate both stage and target before swap.
- Bind the canonical parent and every transaction-owned directory to its
  device/inode identity before each rename, deletion, or lock release.
- Never mutate an existing file in place or follow a link during content reads,
  staging, replacement, cleanup, or rollback.

# Changelog

## 0.1.0 — 2026-09-30

- Add bounded defensive preparation of complete plugin path-and-byte snapshots.
- Validate manifests, selected runtime entries, referenced schemas, signatures,
  trust roots, reviewed grants, and capability templates before filesystem IO.
- Generate deterministic manifest-bound `grants.toml` authority snapshots.
- Restrict verified trust to minimal distributions fully bound by the current
  manifest-plus-selected-entry signature contract.
- Add exclusive same-root locking, private staging, byte-for-byte verification,
  atomic replacement, rollback, immutable mode, cancellation, and exact no-op
  reinstalls.
- Reject unsafe roots and targets, links, non-regular files, identity changes,
  path collisions, and unbounded package or filesystem scans.
- Bound grant preprocessing, package topology, destination names, and existing
  directory-only trees; reserve the full case-folded `grants.toml` namespace.
- Use reversible URI capability path expansion for drive letters, backslashes,
  whitespace, controls, and non-ASCII bytes without POSIX path aliasing.
- Reject shared-writable POSIX roots and existing trees, require an explicit
  Windows root-and-tree ACL verifier, and bind every transaction directory to
  the root owner and filesystem.
- Detect exact and case-folded file/ancestor collisions independent of input
  order before any filesystem operation.

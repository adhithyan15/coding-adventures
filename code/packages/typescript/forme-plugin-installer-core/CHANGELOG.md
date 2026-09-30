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

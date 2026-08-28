### Changed - independent removed-monolith completeness (#13353)

- `check:shards` now proves exact logical-owner completeness for all 47 generic
  removed core-spine, chapter, and curriculum plans from independent
  cross-ledger identities. It detects clean deletion, unexpected and duplicate
  owners, case-fold collisions, and filename/body drift without rejecting a
  stable owner inserted at an intermediate ordinal. The four intentionally
  incomplete script inventories are explicitly structural-only until #13381
  supplies independent per-glyph declarations.

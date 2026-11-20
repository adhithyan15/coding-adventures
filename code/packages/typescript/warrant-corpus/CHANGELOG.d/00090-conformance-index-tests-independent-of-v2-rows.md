## Unreleased - conformance index tests independent of the committed v2-* rows (CV2-S5-1b, D59)

- The index-rule tests no longer assume the committed `index.json` holds
  exactly the four v1 rows. Their scratch copies first strip any `v2-*` rows and
  files, then add only synthetic rows; expected row counts derive from the
  scratch base; the missing-v1-row test removes `label-digest` by name rather than
  popping the last row.
- New simulation test: a fixture that already holds `v2-value` and `v2-arith`
  rows is used as the base and the row-adding, row-removing and ordering
  scenarios pass against it, as they must once the real v2 rows are committed.
- Tests only: no runner source changed, so `runner.src_sha256` and every
  committed manifest are unchanged. No version bump.

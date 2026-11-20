## Unreleased - the conformance test tolerates digest-checked v2-* index rows (CV2-S5-1, D40)

- The conformance test no longer closes the category set of `index.json`. It
  still requires a row for every v1 category it knows (`json`, `tagged`,
  `containment`, `label-digest`, in that order, first), checks the file sha256
  and case count of every row and that `total_cases` is their sum, accepts a row
  named `v2-*` after the v1 rows (the known v2 names in the kit's order) without
  running it, and fails any other unknown row.
- New tests on a temp copy of the fixture: an added `v2-x` row with its file
  passes; an unknown non-`v2-` row, a `v2-` row with a wrong digest, a wrong case
  count or a missing file, a missing v1 row and v2 rows out of order each fail.
- Tests only: no runner source changed, so `runner.src_sha256` and every
  committed manifest are unchanged. No version bump.

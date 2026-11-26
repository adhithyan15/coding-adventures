## Unreleased - three checks VER04r section 16.21 recorded as not built (D73, R107 d, R108)

The kit is unchanged: 782 cases run, 0 failed; the 69 `v2-chunk` cases that passed still pass and the 5 that expect
`chunk-file` keep that outcome. No dependency added.

- **[D73] Chunk `calls` sum.** `chunks.check_chunk_file` refuses, as `chunk-file`, a file whose chunks' `calls` do not
  sum to the row's `expected_calls`, after the `cases`/`discarded` check and before the suite digests.
- **R107 (d) Tag checks before `level-missing`.** `ir.scan_tags` runs the tag-name checks (`unknown-tag`, then
  `subject-forbidden` outside a claim body) over `admit`, `body`, `oracle.expected` and `oracle.subject` at the start of
  `chunks._assemble_suite`, before anything can record `level-missing`. A suite with an `ir.fixed` tag in `admit` and an
  unknown tag in `body`, or a `subject_op` in `admit`, is now `corpus` (R69 e), not `level-missing`.
- **R108 Consumer-side type-table checks.** `wtypes.check_types_table` also refuses a duplicate field or constructor
  name, a field type naming an undeclared type (also under `Option`, `List`, tuples and as a `Result` error), and a
  record or variant that contains itself directly or through any chain (iterative search, no Python recursion). The
  table is refused whole, before any value is read, with the per-context code (`type`, `ill-typed`, `descriptor`,
  `corpus`).

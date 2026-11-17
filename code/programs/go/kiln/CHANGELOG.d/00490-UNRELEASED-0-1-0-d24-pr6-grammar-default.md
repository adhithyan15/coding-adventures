## [0.1.0] - UNRELEASED

### Changed (trace parser default flip, D24 PR 6)

- The grammar path (`strace-lexer` / `strace-parser`, compact `LineParser`) is
  now the production reader of strace lines. `ParseOptions` parses with it, and
  `parseCrossChecked` returns the grammar path's result. For every reason-only
  row of the KILN04 4.9 table, and for every stricter-only refusal, the recorder
  now sees the grammar path's `Reason`. The only trace `Reason` the recorder
  matches on is `helper-no-exec` / `helper-no-init` (model-layer, after pass 1),
  which the flip does not change.
- The old hand-written scanner stays compiled as the ORACLE, reachable only
  through the cross-check (this package's tests, and `KILN_TRACE_DIFF=1` in the
  kiln-sandbox job). Old-refused/new-accepted is still a loud failure. There is
  no runtime or environment switch back to it: that would be a soundness
  downgrade. Rollback is a revert. It is deleted in PR 8 after Phase B and a
  soak (14 days, 200 sandbox runs, zero mismatches and zero allowed divergences
  on real traces, clean nightly fuzz).
- The "at least 8 MB/s and within 3x of the old scanner" flip gate is replaced
  by G1 (blocking: a cap-sized 2 GiB action parses within a quarter of the 10
  minute classify deadline, a floor of 14 MB/s) and G2 (report-only: parse at
  most 10% of step wall time and 5 s at the census p99 trace size).

### Added

- `TestDefaultPathIsTheGrammarPath`: with the cross-check unset, a stricter-only
  input is refused, and a reason-only input carries the grammar path's reason.
- kiln-sandbox: a real trace of 32 KB or more that the grammar path reads below
  14 MB/s fails the job (G1). `PROBE: trace` lines gain `step_wall_ms`,
  `parse_ms` and `parse_share` where the step's wall time is known (G2 census).
  `ci/assert_sandbox_tests.py` prints up to 60 facts of 450 characters.

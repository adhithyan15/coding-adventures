## [0.1.0] - UNRELEASED

### Added (D34: cross-check the traces the record children wrote)

- The kiln-sandbox job's `KILN_TRACE_DIFF=1` reached only the `go test` process;
  the real parses of `kiln record prepare`, `step`, the classify child and
  `collect` ran grammar-only, so the 14-day / 200-run soak saw almost no real
  traces. The variable is not passed to children (their environment is sealed
  into facts and keys).
- `trace.ParseCrossChecked` (test instrument, guarded by
  `TestMeasureIsNotUsedByProductionCode`) returns a cross-check mismatch as a
  value and the number of allowed divergences of one parse.
  `record.ProbeParseLimits` exports the probe limits.
- The step, collect, gates, survivor and probe sandbox tests register
  `crossCheckChildTraces`: when the test ends it re-parses every `*.trace` under
  the record directory with the child's Limits, helper exe and cwd, fails on any
  mismatch or allowed divergence, and logs `PROBE: crosscheck traces=N bytes=B
  mismatches=M allowed=A` (`accepted` shows a trace really parsed; with a
  verified `pre/` the test's verdict must match the child's, probe runs and
  `parse_ok` step traces must be accepted, steps come from the directory
  listing, walk errors fail). Registered before `prepare` runs, so a probe trace
  prepare refuses is still checked.
- `ci/assert_sandbox_tests.py` requires such lines with traces >= 1 and
  accepted >= 1 in total from passing tests. Its tests are in
  `code/scripts/tests/test_assert_kiln_sandbox_tests.py` (unittest), which no
  workflow ran before: the kiln-sandbox job now does, and `ci-gates.json` lists
  the file as a path of that job.
- KILN04 section 4.9: the D34 entry and the soak counting.

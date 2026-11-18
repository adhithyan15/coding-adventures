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
- Gate G1 is enforced only in the dedicated kiln-sandbox job, via the test-only
  env var `KILN_G1_GATE=1` (set in `ci.yml` next to `KILN_TRACE_DIFF`). The
  general build job runs kiln's tests in parallel with many other packages and
  measured the synthetic slice at 11.2 MB/s, so elsewhere the same PROBE lines
  are logged with `gated=0` and no floor (zero time / non-finite rate still
  fail). First dedicated-job figures: synthetic-slice 21.6 (26.04) and 18.7
  (24.04) MB/s, real traces 28-37.
- kiln-sandbox gate G1 (KILN04 4.9): a real trace of 32 to 128 KB whose grammar
  path pass 1 is below 14 MB/s fails the job; larger and smaller traces are
  census only (`gated=0`). The gated figure is the best (minimum) of at least 7
  passes spanning at least 250 ms after a warm-up, not a single shot (a ~2 ms
  sample flaked to 5-7 MB/s under contention), with the collector off for traces
  of 128 KB or less (production runs with it on; the honest margin over the
  floor is about 1.3x). `trace.MeasureBest` / `trace.BestPass1` provide it; they
  toggle a process-global GC setting and are CI/test-only
  (`TestMeasureIsNotUsedByProductionCode`). It is a proxy for the 2 GiB cap.
- `TestSandboxTraceGateSynthetic`: a 64 KB slice of repeated `build.strace`,
  gated best-of-7 against the same floor, logged as `PROBE: trace
  synthetic-slice`; the fixture repeated to 4 MiB is read in full and logged on
  its own `synthetic-4mib-census` line (`gated=0`). A zero best time or a
  non-finite rate fails.
- `PROBE: trace` lines gain `best_mb_s` and `gated=0|1` (single-shot
  `grammar_mb_s` stays for the census) and, where the step's wall time is known
  (only the signals test today), `step_wall_ms`, `parse_ms`, `parse_share` for
  the G2 census (report-only; needs wall-time plumbing before it can block).
- `ci/assert_sandbox_tests.py` fails the job unless a passing test logged a
  `gated=1` PROBE line, `synthetic-slice` from `TestSandboxTraceGateSynthetic`
  is among them, and every gated `best_mb_s` is finite, positive and at least
  14. A PROBE line counts only when its payload starts with `PROBE: trace `
  after the go test `file:line` prefix. It prints up to 60 facts of 450
  characters. New pytest cases.
- KILN04 4.9: G1 is stated as a pass-1 rate; a stale "uploads an artifact on any
  mismatch" sentence is corrected (ci.yml uploads none).

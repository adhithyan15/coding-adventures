## [0.1.0] - UNRELEASED

### Changed (G2 census, PR A: PROBE line hygiene and wall time)

- `PROBE: trace` lines now put the two `%q` refusal fields (`old_refusal=`,
  `grammar_refusal=`) LAST. Every number the CI script trusts now comes before
  any text a trace could influence.
- `ci/assert_sandbox_tests.py` reads `key=value` facts only from the text before
  the first refusal field, and the first occurrence of a key wins, so a refusal
  that quotes `best_mb_s=99 gated=1` can neither override nor add a fact. The
  PROBE line anchor is now an exact indent (four spaces per level of subtest
  nesting, which is what `go test` writes) instead of `\s*`, so an indented
  continuation line of multi-line output cannot match. New pytest cases: a
  refusal quoting fields, a duplicate key, a continuation line, a subtest
  indent, and well-formed lines still parsing.
- Wall time on more PROBE lines (KILN04 4.9, G2 census, report-only): the
  record-step sandbox test takes the step's wall time from the step sidecar's
  `started_at`/`ended_at` and logs `step_wall_ms`, `parse_ms`, `parse_share`
  through `probeTraceWall`. `record.ProbeRunRecord` gains an in-memory
  `Wall time.Duration` (`json:"-"`, set by `runProbe`) so the probe path can do
  the same; a new in-process subtest of `TestSandboxRecordProbe` logs it. The
  duration is never sealed: `TestProbeWallIsNeverSealed` encodes two reports that
  differ only in `Wall` and requires identical bytes with no duration in them,
  and the sandbox subtest checks the real `pre/probes.json`.
- A non-positive or unparsable wall is dropped, never used as a divisor, and
  counted (`walls_dropped=N` on the lines that follow). Portable tests for the
  line shape and the wall helpers live in `probe_line_test.go`.

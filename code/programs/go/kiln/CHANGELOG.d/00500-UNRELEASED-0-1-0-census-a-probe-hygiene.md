## [0.1.0] - UNRELEASED

### Changed (G2 census, PR A: PROBE line hygiene and wall time)

- `PROBE: trace` lines now put the two `%q` refusal fields (`old_refusal=`,
  `grammar_refusal=`) LAST. Every number the CI script trusts now comes before
  any text a trace could influence.
- `ci/assert_sandbox_tests.py` reads `key=value` facts only from the text before
  the first refusal field, and the first occurrence of a key wins, so a refusal
  that quotes `best_mb_s=99 gated=1` can neither override nor add a fact. The
  PROBE line anchor is now exactly four spaces (what `go test -v -json` writes
  for every `t.Logf` line at any subtest depth) instead of `\s*`, so an
  eight-space continuation line cannot match. Output is buffered per test and
  only newline-terminated lines count, because `test2json` splits long lines
  into 1024-byte events with no newline at the cut. A label ending in `:` is
  rejected, and the open/measure failure lines are now `PROBE: trace-error`
  with the error `%q`-quoted. New pytest cases: a refusal quoting fields, a
  duplicate key, a continuation line, subtest indents, a verbatim real
  `-v -json` event, a line split across events, a forged line after a split, a
  colon label, and well-formed lines still parsing.
- The in-process probe wall subtest logs `probe-wall-<name>` labels (not
  `probe-<name>`), checks its report with `CheckProbeReport` and against the
  sealed run names.
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

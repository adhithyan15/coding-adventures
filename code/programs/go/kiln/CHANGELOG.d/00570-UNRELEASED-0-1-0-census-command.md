## [0.1.0] - UNRELEASED

### Added (census PR C: `kiln record census`)

- `kiln record census --dir <abs> --out <abs> [--md <abs>] [--records <abs>]`, a
  diagnostic that reads a finished record directory back and writes a bounded,
  sanitised summary (schema `kiln-census-summary/1`, JSON at most 512 KiB, an
  optional Markdown rendering at most 256 KiB for `$GITHUB_STEP_SUMMARY`). It
  is never run during a recording; a later manual workflow runs it after a
  sampled real build. Per action it reads the step traces and sidecars,
  `census.json`, `record.json` / `record.failed` and, with `--records`,
  collect's header and record count.
- Reported: the host-path reason histogram (by kind, by path prefix of depth at
  most 4, and the top 200 full paths per kind); trace-size distributions
  (p50/p90/p99/max and log2 buckets) for all steps and for steps that exited 0;
  G2 timing (step wall, one timed grammar parse through `parseStep`, GC on, and
  their share, with the counts over 10% and over 5 s); each action's total
  trace bytes against the 2 GiB cap; the cross-check line (traces, bytes,
  accepted, mismatches, allowed divergences); the recordable share per
  language. A missing census, an empty census beside a failed parse half, an
  invalid census and `record.failed` are each counted separately and never read
  as "no host reads".
- Every input is read as hostile: opened without following symlinks and without
  blocking on a FIFO, regular files only, size-bounded, strictly decoded (no
  unknown, duplicate or case-variant key; schema, action, kind and counts
  checked), and skipped with a counted problem code on any failure. Paths are
  sanitised, redacted (`$HOME`, `$TMPDIR`, `$RUNNER_TEMP`, the workspace, the
  record directory) and cut to 512 runes. In the Markdown every path is in a
  code span longer than any backtick run in it, other text is escaped, no table
  cell holds attacker text, no raw control character occurs and no line starts
  with `::`.
- Exit status: 0 clean; 4 a cross-check mismatch or any allowed divergence
  (strict); 1 infrastructure failure (no traces, an unverifiable `pre/`, an
  output that cannot be written) or a refusal (not a record directory, an output
  that exists or lies inside `--dir`; a refusal writes nothing); 2 usage. The
  summary is written whenever the directory was read. Outputs are created
  `O_EXCL`, mode 0600.

### Changed

- `trace.ParseCrossChecked` is now serialised by a mutex and is allowed in
  production code, for `kiln record census` only: it never touched the GC (the
  reason `trace.Measure*` are test-only), only a process-global counter.
  `TestMeasureIsNotUsedByProductionCode` no longer lists it;
  `TestParseCrossCheckedOnlyInCensus` allows one caller,
  `internal/record/census_report.go`.
- `parseHelperTrace` builds its parse options through `helperParseOptions`, the
  function the census uses, so the two can never drift apart.
- KILN04 gains the "`kiln record census`" section (inputs, schema, bounds,
  exit codes, security). `TestSandboxRecordCollect` also runs the command over
  its real recording.

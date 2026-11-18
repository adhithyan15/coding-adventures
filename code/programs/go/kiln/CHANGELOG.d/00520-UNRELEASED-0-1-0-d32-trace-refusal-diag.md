## [0.1.0] - UNRELEASED

### Added (D32: diagnose a refused real trace)

- A kiln-sandbox CI run failed `TestSandboxRecordStep` with `probe walk: the
  trace does not parse: trace-incomplete:unparsable-line`. The trace was deleted
  with the test's temp dir, and the children that run `prepare` have no
  `KILN_TRACE_DIFF`, so the parse was grammar-only: no way to tell which line was
  refused or whether the old scanner agreed. (The likely cause is a
  timing-dependent strace thread-death line such as
  `NAME(args <unfinished ...>) = ?`, which both parsers refuse today.)
- `prepare`'s refusal now ends with ` (line N)`, the refused trace line, in the
  display text only. The reason string is unchanged because it feeds the
  sidecar's `parse_reasons`; a line number carries no path text.
  `parseHelperTrace` returns the line as a third result.
- `trace.Diagnose(data, helperRoot)` (measure.go; test/CI only, listed in
  `TestMeasureIsNotUsedByProductionCode`) runs pass 1 with both readers and
  reports reasons, error and stop lines, the `grammarPanics` delta, the line
  count and trailing newline; under `KILN_TRACE_DIFF=1` also a sanitised window
  of 3 lines either side of the earlier stop line plus the stop pid's last
  earlier line and a reader's Error line when it differs from its stop line.
  Key lines show their first and last 160 bytes and come before the window;
  the report notes it covers pass 1 only. Capped at 8 KB.
- Step and classify-child refusals (sidecar `parse_reasons`, not stderr) are
  covered too: `logStepTraces` diagnoses an action's step traces
  (`record.StepFile`), called by the step, collect and gates sandbox tests when
  a parse reason or an unexpected unknown write appears. The file is opened once
  with `O_NOFOLLOW|O_NONBLOCK`, only the quoted base name is logged, and a read
  cut at 8 MiB says so.
- The linux sandbox tests' `recStepRun` logs `Diagnose` for a child that exits
  non-zero with `the trace does not parse ... (trace: <path>)` (absolute path
  under the temp dir, regular file, at most 8 MiB). No artifact is uploaded: the
  repository is public and traces hold paths (KILN04 section 4.9).

# Changelog — iir-coverage-lcov

## [0.1.0] — 2026-04-30

Initial release.  lcov info-format exporter for
`iir_coverage::LineCoverageReport`.  Drop-in CI integration with
the lcov ecosystem (`genhtml`, Codecov, SonarQube, GitLab CI's
coverage parser, every lcov-aware tool).

### Added

- `to_lcov(report)` — single public function.  Returns a `String`
  of valid lcov info-format text.  One record per source file in
  the report (sorted by path), `DA:` lines sorted by line ascending,
  with `LF:` / `LH:` totals and the `end_of_record` terminator.
  Output ends with a final newline so it concatenates safely.

### Hardening

- **`SF:` path sanitisation.**  lcov has no escape mechanism, so a
  path containing `\n`, `\r`, or `end_of_record` could let an
  attacker who controls `iir_coverage::CoveredLine.file` inject
  fake coverage records.  This crate replaces `\n`/`\r` with `_`
  and disarms the `end_of_record` substring.  Three dedicated
  tests lock the contract in.

### Notes

- Pure data → text.  No I/O, no FFI, no unsafe.  Single dep on
  `iir-coverage` (also empty capabilities).  See
  `required_capabilities.json`.
- 11 unit tests covering empty case, single- / multi-line records,
  IIR-hit-count aggregation, sort order, format-prefix shape,
  trailing-newline contract, benign-Unicode passthrough, and the
  three injection-defense tests.
- Filed as follow-ups in the README roadmap: JSON exporter
  (`iir-coverage-json`), Cobertura XML exporter
  (`iir-coverage-cobertura`), terminal pretty-printer
  (`iir-coverage-terminal`).

# Changelog — iir-coverage

## [0.1.0] — 2026-04-30

Initial release.  **LANG dev-tools D-4** — IIR-level coverage
projection.  First consumer of the `IIRFunction.source_map`
populated by D-1 (PR #1834).

### Added

- `CoveredLine { file, line, iir_hit_count }` — one source line
  reached during execution.  `iir_hit_count` is the number of
  *distinct* IIR instructions at that source line that ran (not
  an execution frequency — see crate docs).

- `LineCoverageReport` — the projection result.  Methods:
  `covered_lines()`, `lines_for_file(path)`,
  `total_lines_covered()`, `files()`, `total_iir_hits()`.

- `ExecutionTrace = HashMap<String, HashSet<usize>>` — the trace
  shape that LANG18-compliant dispatchers (vm-core / lispy-runtime
  / twig-vm) produce.

- `build_report(module, trace, source_file) -> Result<LineCoverageReport, CoverageError>`
  — the projection.  Walks the trace, indexes functions by name,
  drops synthetic positions (`SourceLoc::SYNTHETIC`), de-duplicates
  hit IPs per line, and aggregates into the report sorted by line
  ascending.

- `CoverageError` — `UnknownFunction`, `IpOutOfBounds`,
  `SourceMapDriftedFromInstructions`.  All three carry diagnostic
  context (function name, IP, instruction counts).  Implements
  `Display` + `Error`.

- `iir_hit_count` saturates at `u32::MAX` rather than silently
  wrapping if a single source line had >4B distinct IIR
  instructions reach it (security-review hardening).

- 16 unit tests covering empty cases, basic projection, sorting,
  synthetic-position dropping, `iir_hit_count` aggregation,
  multi-function lines, query helpers (`lines_for_file`, `files`),
  all three `CoverageError` variants, error display, and a
  realistic two-function trace.

### Notes

- Pure data + algorithms.  Single dependency on `interpreter-ir`
  (path), itself capability-empty.  No I/O, no FFI, no unsafe.
  See `required_capabilities.json`.
- Filed as follow-ups: dispatcher-side hook (vm-core / lispy-runtime /
  twig-vm `enable_coverage` API), JSON / lcov / cobertura exporters,
  terminal formatter, LSP code-lens provider, multi-file overload,
  memory tightening of the dedup key.
- Security review found no HIGH or CRITICAL issues.  Two MEDIUM
  follow-ups (saturating cast, dedup-key string-clone memory) — the
  saturating cast is fixed in this PR; the memory follow-up is
  documented in the README roadmap.

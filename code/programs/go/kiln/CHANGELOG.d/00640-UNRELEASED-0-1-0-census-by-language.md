## [0.1.0] - UNRELEASED

### Added (census reporting: parse refusals by language and call, KILN04)

Reporting only: record.json, records.jsonl, the binding and every verdict are
byte-identical (`TestCensusDoesNotChangeTheRecord`, extended).

- `kiln record census` summary, additive JSON under `explain`:
  `record_parse_reasons_by_language`, `parse_refusals_by_language`,
  `parse_refusal_syscalls` (reason x syscall) and
  `parse_refusal_syscalls_by_language`. The Markdown gains "Parse reasons by
  language" and "Parse refusals by system call", every name in a code span.
- `trace.Error.Call`: the closed-table syscall a refusal was raised for (set in
  `model.call`'s `fail`, only when `trace.SyscallClass` knows the name). Never in
  `Error()` or a record. This is what tells `cwd-race` chdir from fchdir and
  `fd-path` fchmod from the rest; `refused:<name>` uses its own name; no call
  means `other`.
- Census op classes also on `trace-incomplete:proc-path` and each
  `trace-incomplete:unstable-path:<sub>` (the `ops` map and `reasons[].ops`). The
  sub-codes raised after the walk use the class of the first look of the path,
  remembered per path only when a census is attached.
- Tests: `internal/trace/errcall_test.go`, `internal/traced/census_ops_test.go`,
  `internal/record/census_language_test.go`, `census_language_unix_test.go`.
- KILN04 census section updated for each new field.

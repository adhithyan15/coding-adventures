# Changelog — iir-coverage-json

## [0.1.0] — 2026-04-30

Initial release.  JSON exporter for `iir_coverage::LineCoverageReport`.
Stable structured-data sibling of `iir-coverage-lcov`.

### Added

- `to_json(report)` — single public function.  Returns a `String` of
  valid JSON.  Schema documented at crate level; `SCHEMA_VERSION = 1`.
- `SCHEMA_VERSION` constant — bumped on incompatible changes.

### Schema (v1)

```json
{
  "schema_version": 1,
  "files": [
    {
      "path": "src/main.twig",
      "lines_found": 42,
      "lines_hit": 37,
      "lines": [{ "line": 1, "iir_hit_count": 3 }, ...]
    }
  ]
}
```

`files` sorted by `path` ascending; `lines` sorted by `line`
ascending.  Output is deterministic for a given input.

### Hardening

- `push_json_string` is the only place untrusted text crosses into
  the output, so it's where injection defences live.  Per RFC 8259:
  `"` and `\` always escaped, `\n`/`\r`/`\t`/`\b`/`\f` shorthand,
  other ASCII control chars escaped as `\u00XX`, non-ASCII UTF-8
  passes through verbatim.  A path containing `\n`, `"`, control
  chars, or even `},{` cannot break the JSON envelope or inject
  extra records.  Three dedicated tests lock the contract in.

### Notes

- Pure data → text.  No I/O, no FFI, no unsafe.  Single dep on
  `iir-coverage` (also empty capabilities).  See
  `required_capabilities.json`.
- No `serde_json` dep — schema small enough that hand-rolled is
  ~50 LOC and avoids 400+ KLOC of transitive deps.
- 10 unit tests covering empty case, single-file projection,
  multi-IIR-per-line aggregation, sort order, JSON-escape
  correctness, Unicode passthrough, schema-version emission,
  compact-output check, and a realistic two-function report.
- Filed as follow-ups in the README roadmap: Cobertura XML exporter,
  terminal pretty-printer, schema v2 once multi-file `build_report`
  lands.

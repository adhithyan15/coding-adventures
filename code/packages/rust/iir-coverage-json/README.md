# iir-coverage-json

**JSON exporter for `iir-coverage::LineCoverageReport`.**

Stable structured-data sibling of [`iir-coverage-lcov`](../iir-coverage-lcov/).
Use whichever fits your downstream tool: lcov for `genhtml` /
Codecov / SonarQube; JSON for in-house dashboards, custom CI gates,
JS/TS pipelines, or any tool that already speaks JSON.

## Schema (v1)

```json
{
  "schema_version": 1,
  "files": [
    {
      "path": "src/main.twig",
      "lines_found": 42,
      "lines_hit": 37,
      "lines": [
        { "line": 1, "iir_hit_count": 3 },
        { "line": 2, "iir_hit_count": 1 }
      ]
    }
  ]
}
```

- `schema_version` — bumped on incompatible changes; consumers
  should refuse unknown values.  Exposed as `SCHEMA_VERSION`.
- `files` sorted by `path` ascending; `lines` sorted by `line`
  ascending.  Output is deterministic for a given input.
- `iir_hit_count` is the number of *distinct IIR instructions at
  this source line that ran* — see [`iir-coverage`](../iir-coverage/)
  for the granularity contract.

## Public API

```rust
pub const SCHEMA_VERSION: u32 = 1;
pub fn to_json(report: &LineCoverageReport) -> String;
```

That's it.  Returns a `String` of valid JSON; caller writes it to
disk or pipes it to whatever consumer.

## Why no `serde_json`?

The schema is small enough that hand-rolled is ~50 LOC and avoids
pulling 400+ KLOC of transitive deps just to format a report.
Coding-adventures crates default to **zero dependencies**.

## Hardening

`push_json_string` is the only place untrusted text crosses into
the output, so it's where injection defenses live.  Per RFC 8259:

- `"` and `\` always escaped.
- `\n`, `\r`, `\t`, `\b`, `\f` escaped as their shorthand forms.
- All other ASCII control characters (` ` to ``,
  except those above) escaped as `\u00XX`.
- Non-ASCII UTF-8 passes through verbatim (RFC 8259 §7 allows this).

This means a path containing `\n`, `"`, control characters, or
even `},{` cannot break the JSON envelope or inject extra records.
Three dedicated tests lock the contract in.

Output is compact (no whitespace).  Pipe through `jq .` for human
reading.

## What this crate does NOT do

- No I/O — returns `String`; caller writes to disk.
- No file-existence checks.
- No function- or branch-coverage records (out of scope; would
  need a different trace shape).

## Example

```rust
use iir_coverage::{build_report, ExecutionTrace};
use iir_coverage_json::to_json;

let report = build_report(&module, &trace, "src/main.twig").unwrap();
let json = to_json(&report);
std::fs::write("coverage.json", json).unwrap();
```

## Dependencies

- `iir-coverage` (path) — the report type.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

## Tests

10 unit tests covering empty case, single-file projection,
multi-IIR-per-line aggregation, sort order, JSON-escape correctness
(quote / backslash / newline / tab / CR / control chars), Unicode
passthrough, schema-version emission, compact output (no
whitespace), and a realistic two-function report.

```sh
cargo test -p iir-coverage-json
```

## Roadmap

- **Cobertura XML exporter** — `iir-coverage-cobertura` for
  Jenkins, Azure DevOps, etc.
- **Terminal pretty-printer** — `iir-coverage-terminal` for the
  CLI / `lang-vm test` integration.
- Schema v2 once the multi-file `build_report_multi_file` lands in
  `iir-coverage`.

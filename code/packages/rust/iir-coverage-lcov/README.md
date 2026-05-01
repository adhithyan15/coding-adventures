# iir-coverage-lcov

**lcov-format exporter for `iir-coverage::LineCoverageReport`.**

Drop-in CI integration with the lcov ecosystem (`genhtml`, Codecov,
SonarQube, GitLab CI's coverage parser, every lcov-aware tool).

---

## Why lcov

lcov is the de-facto interchange format for line-coverage data.
Every coverage tool either consumes lcov directly (`genhtml`) or
accepts it as one of its input formats (Codecov, Coveralls,
SonarQube).  Producing lcov gives the LANG-VM coverage stack
immediate compatibility with the entire downstream tooling
ecosystem without us writing per-tool adapters.

## Public API

```rust
pub fn to_lcov(report: &iir_coverage::LineCoverageReport) -> String;
```

That's it.  Returns a `String` that the caller writes to disk
(or pipes through `genhtml`, etc.).

## Format produced

```text
TN:
SF:<file path>
DA:<line>,<exec count>
DA:<line>,<exec count>
...
LF:<lines found>
LH:<lines hit>
end_of_record
```

One record per distinct source file in the report (sorted by path
ascending).  Within each record, `DA:` lines are sorted by source
line ascending.  `iir_hit_count` from `iir_coverage::CoveredLine`
becomes the `<exec count>` on each `DA:` line — recall that this
is the number of distinct IIR instructions at that source line
that ran (not an execution-frequency).  `genhtml` and similar tools
display it as "hit count" — close enough for the typical "is this
line covered?" question that motivates running coverage in the
first place.

The output ends with a final newline so it can be concatenated
with other lcov files (e.g. `cat report1.lcov report2.lcov | genhtml`).

## Hardening (security review)

lcov has **no escape mechanism** for `SF:<path>` lines.  A path
containing `\n` (legal on POSIX) or `\r` could let an attacker
who controls the `source_file` argument to
`iir_coverage::build_report` forge extra coverage records for
arbitrary files, potentially fooling downstream tools (Codecov,
SonarQube) that gate merges on coverage deltas.

This crate replaces any `\r` or `\n` byte in a path with `_`, and
disarms the `end_of_record` substring (becomes `end_of_record_`),
so injection is impossible.  Three dedicated tests
(`newline_in_path_replaced_with_underscore`,
`carriage_return_in_path_replaced_with_underscore`,
`end_of_record_substring_in_path_disarmed`) lock the contract in.

All other characters (spaces, ampersands, Unicode) pass through
verbatim — lcov consumers handle them fine.

## What this crate does *not* do

- **No I/O.**  Returns a `String`; the caller writes it to disk.
  Keeps the crate capability-free and trivially testable.
- **No file-existence checks.**  We don't try to open `SF:` paths.
  lcov consumers do that themselves.
- **No function- or branch-coverage records.**  Out of scope by
  design (would need a separate trace shape).  See
  [`iir-coverage`](../iir-coverage/) for why per-line frequency /
  branch coverage live in different layers.

## Example

```rust
use iir_coverage::{build_report, ExecutionTrace};
use iir_coverage_lcov::to_lcov;

// (build a LineCoverageReport via iir_coverage::build_report — see that
// crate's docs)
let report = build_report(&module, &trace, "src/main.twig").unwrap();
let lcov = to_lcov(&report);

// Write it for genhtml / Codecov / etc.
std::fs::write("coverage.lcov", lcov).unwrap();
```

## Dependencies

- `iir-coverage` (path) — the report type.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

## Tests

11 unit tests covering the empty case, single- / multi-line
records, IIR-hit-count aggregation, sort order, format-prefix
shape, the trailing-newline contract, benign-Unicode passthrough,
and the three injection-defense tests above.

```sh
cargo test -p iir-coverage-lcov
```

## Roadmap

- **JSON exporter** — `iir-coverage-json` for tools that prefer
  structured input (parallel to lcov).
- **Cobertura XML exporter** — `iir-coverage-cobertura` for
  Jenkins, Azure DevOps, etc.
- **Terminal pretty-printer** — `iir-coverage-terminal` for the
  CLI / `lang-vm test` integration.

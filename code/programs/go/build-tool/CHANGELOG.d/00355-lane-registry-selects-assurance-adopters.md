## Unreleased

- A change to `code/specs/fixtures/ct-compare-warrant-v1/consumers.json` now also selects the six Rust packages that use `ca-assurance-report` (arithmetic, ct-compare, gf256, logic-gates, lzw, wasm-leb128), whose `assurance_report.json` lists a rung per registry lane (WD-004b2). New test `TestLaneRegistryEditSelectsEveryAssuranceReportAdopter`.

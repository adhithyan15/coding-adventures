---
category: Rust
---

# A rust package with an assurance_report.json is not necessarily a user of ca-assurance-report, so list adopters by BUILD line, not by file

WD-004b2 added a required `lanes` field to the report that `ca-assurance-report`
generates, so "regenerate every adopter" was the next step. `ls rust/*/assurance_report.json`
lists 32 files; looping `--generate` over all of them failed for 26 with
`cannot read ./evidence/test-manifest.json`. Those 26 (games, simulators, codecs) carry a
different, hand-written report schema (no `commit`, no `catalogue` pin of this tool) and no
`evidence/` directory: nothing in this repo reads them with this tool. Only six packages
(arithmetic, ct-compare, gf256, logic-gates, lzw, wasm-leb128) run
`cargo run -p coding_adventures_ca_assurance_report -- --check` in their BUILD.

Do differently: find a tool's adopters by the line that runs it
(`grep -l 'ca_assurance_report' */BUILD*`), not by the artifact's file name, and check
that the generated file's schema is the tool's before treating a failed `--generate` as a
bug. Regenerate with the existing `--commit` (read it from the old report) so the diff shows
only the field you meant to add, and compare old and new as JSON to list the changed fields.

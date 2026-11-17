## Unreleased — TRIG-REF-1 corpus consumer for c/trig (TC-4)

- `c/trig` joins `python/trig`, `rust/trig` and `typescript/trig` as a consumer of
  `code/specs/fixtures/trig-reference-v1/`: its conformance test decodes every row, so an
  edit to any file of the fixture selects the package. A `-language c` filter selects only
  `c/trig`; `go/trig` (not yet converged) stays unselected.
- `TestTrigReferenceCorpusSelectsConvertedPorts` covers the new row and filter.

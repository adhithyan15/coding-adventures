## Unreleased — TRIG-REF-1 corpus consumers for rust/trig and typescript/trig (TC-2)

- `rust/trig` and `typescript/trig` join `python/trig` as consumers of
  `code/specs/fixtures/trig-reference-v1/`: their conformance tests decode every row, so an
  edit to any file of the fixture selects each of the three packages. A `-language`
  filter selects only that language's package; `go/trig` (not yet converged) stays
  unselected.
- `TestTrigReferenceCorpusSelectsConvertedPorts` covers the three rows and the filters.

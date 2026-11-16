## Unreleased — TRIG-REF-1 corpus consumer for cpp/trig (TC-5)

- `cpp/trig` joins `python/trig`, `rust/trig` and `typescript/trig` as a consumer of
  `code/specs/fixtures/trig-reference-v1/`: its `tests/conformance_test.cpp` decodes every
  row, so an edit to any file of the fixture selects each of the four packages. A
  `-language cpp` run selects only `cpp/trig`; `go/trig` (not yet converged) stays
  unselected. `cpp/trig` is not a consumer of the generator, oracle or validator paths:
  only the `python/trig` BUILD runs those.
- `TestTrigReferenceCorpusSelectsConvertedPorts` covers the four rows and the filters.

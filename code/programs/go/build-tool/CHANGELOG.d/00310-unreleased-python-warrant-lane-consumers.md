## Unreleased — Python Warrant lane repository inputs (VER04i, WD-101)

- Two repository input consumers: `python/warrant-corpus` (its tests run the
  real `ct-compare` corpus) and `python/ct-compare` (its BUILD re-runs the
  corpus, fails if the committed lane manifest drifted, and validates it
  against the consumer registry). Both are selected by an edit to
  `code/specs/ct-compare.warrant`, `cases.tfv`, `corpus.json` or
  `consumers.json` under `code/specs/fixtures/ct-compare-warrant-v1/`;
  `python/ct-compare` is also selected by any edit beneath
  `code/packages/python/warrant-corpus/`, because it installs that runner as a
  sibling editable dev dependency, which is not a build-graph edge.
- `TestWarrantLaneArtifactsSelectGrammarAndAssuranceConsumers` now lists the
  two Python packages (a missing consumer fails closed) and
  `TestPythonWarrantLaneInputsSelectTheirConsumers` pins the selection, the
  language filter, the missing-consumer failure and the lookalike paths.

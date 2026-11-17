## Unreleased — Go Warrant lane repository inputs (VER04i, WD-103)

- Two repository input consumers: `go/warrant-corpus` (its tests run the real
  `ct-compare` corpus) and `go/ct-compare` (its BUILD re-runs the corpus, fails
  if the committed lane manifest drifted, proves every negative control and
  validates the manifest against the corpus). Both are selected by an edit to
  `code/specs/ct-compare.warrant`, `cases.tfv`, `corpus.json` or `consumers.json`
  under `code/specs/fixtures/ct-compare-warrant-v1/`; `go/ct-compare` is also
  selected by any edit beneath `code/packages/go/warrant-corpus/`. The pilot's
  `go.mod` also requires the runner (a build-graph edge), but its manifest embeds
  the runner's source digest, so the selection does not rely on the resolver.
- `TestWarrantLaneArtifactsSelectGrammarAndAssuranceConsumers` now lists the two
  Go packages (a missing consumer fails closed) and
  `TestGoWarrantLaneInputsSelectTheirConsumers` pins the selection, the language
  filter, the missing-consumer failure and the lookalike paths.

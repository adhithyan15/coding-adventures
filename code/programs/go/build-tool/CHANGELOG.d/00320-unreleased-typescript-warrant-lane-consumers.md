## Unreleased — TypeScript Warrant lane repository inputs (VER04i, WD-102)

- Two repository input consumers: `typescript/warrant-corpus` (its tests run the
  real `ct-compare` corpus) and `typescript/ct-compare` (its BUILD re-runs the
  corpus, fails if the committed lane manifest drifted, proves every negative
  control and validates the manifest against the corpus). Both are selected by
  an edit to `code/specs/ct-compare.warrant`, `cases.tfv`, `corpus.json` or
  `consumers.json` under `code/specs/fixtures/ct-compare-warrant-v1/`;
  `typescript/ct-compare` is also selected by any edit beneath
  `code/packages/typescript/warrant-corpus/`. The pilot also names the runner as
  a `file:` devDependency (a build-graph edge), but its manifest embeds the
  runner's source digest, so the selection does not rely on the resolver seeing
  a dev-only edge.
- `TestWarrantLaneArtifactsSelectGrammarAndAssuranceConsumers` now lists the two
  TypeScript packages (a missing consumer fails closed) and
  `TestTypeScriptWarrantLaneInputsSelectTheirConsumers` pins the selection, the
  language filter, the missing-consumer failure and the lookalike paths.

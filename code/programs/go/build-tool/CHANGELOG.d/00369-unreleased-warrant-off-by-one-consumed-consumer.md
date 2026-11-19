## Unreleased — the oracle's off-by-one-consumed fixture selects rust/warrant-corpus-gen (CV2-S10o-1)

- `isWarrantCorpusGenInput` and `warrantCorpusGenPaths` gain
  `code/packages/python/warrant-oracle/tests/fixtures/off-by-one-consumed-v1/` (the oracle's synthetic
  `off-by-one-consumed` document, its profile and its committed faults and refusals dumps). The generator's slice
  S10o-2 reproduces the dumps byte for byte, reading them from the oracle's directory, so an edit to any file there
  reruns the generator's tests (the same pattern as the tuple-forms and fixed-eval fixtures). Directories match by
  slash-delimited prefix (`off-by-one-consumed-v10/` is not it). The row's `inputs` text names the fixture.
- `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves the files of the directory select
  `rust/warrant-corpus-gen` (and the `.warrant`, being a Warrant document, also `rust/warrant-grammar`), and that the
  lookalikes select nothing.

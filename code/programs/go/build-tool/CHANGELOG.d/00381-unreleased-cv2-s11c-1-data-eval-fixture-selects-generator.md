## Unreleased — the oracle's data-eval fixture selects rust/warrant-corpus-gen (CV2-S11c-1)

- `isWarrantCorpusGenInput` and `warrantCorpusGenPaths` gain `warrantDataEvalFixture`,
  `code/packages/python/warrant-oracle/tests/fixtures/data-eval-v1/` (the oracle's synthetic `ir.data` evaluation
  document of slice S11c-1, its profile and its six dumps: lowering, evaluation, faults, values, domains and refusals).
  It is the generator's byte-parity target for slice S11c-2, which reads the dumps from the oracle's directory, so an
  edit to any file there reruns the generator's tests (the same pattern as the data-forms fixture). Directories match
  by slash-delimited prefix (`data-eval-v10/` is not it). The generator row's `inputs` text names the fixture.
- `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves the files of the directory select
  `rust/warrant-corpus-gen` (and the `.warrant`, being a Warrant document, also `rust/warrant-grammar`), and that the
  lookalikes select nothing.

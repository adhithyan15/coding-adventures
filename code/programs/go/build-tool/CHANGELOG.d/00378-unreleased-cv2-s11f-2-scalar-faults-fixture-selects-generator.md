## Unreleased — the oracle's scalar-faults fixture selects rust/warrant-corpus-gen (CV2-S11f-2)

- `isWarrantCorpusGenInput` and `warrantCorpusGenPaths` gain
  `code/packages/python/warrant-oracle/tests/fixtures/scalar-faults-v1/` (the oracle's synthetic document for
  `saturate-add`, `saturate-sub`, `shift-mask` and the scalar `missing-error`, its profile and its committed
  faults and refusals dumps). The generator's `tests/scalar_faults_parity.rs` reproduces the dumps byte for byte,
  reading them from the oracle's directory, so an edit to any file there reruns the generator's tests (the same
  pattern as the off-by-one-consumed fixture). Directories match by slash-delimited prefix (`scalar-faults-v10/`
  is not it). The row's `inputs` text names the fixture.
- `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves the files of the directory select
  `rust/warrant-corpus-gen` (and the `.warrant`, being a Warrant document, also `rust/warrant-grammar`), and that
  the lookalikes select nothing.

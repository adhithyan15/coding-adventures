## Unreleased — the oracle's data-forms fixture selects rust/warrant-corpus-gen (CV2-S11b)

- `isWarrantCorpusGenInput` and `warrantCorpusGenPaths` gain
  `code/packages/python/warrant-oracle/tests/fixtures/data-forms-v1/` (the oracle's synthetic `ir.data` document
  of slice S11a, its lowering dump and its refusals dump). The generator's `tests/data_parity.rs` reproduces the
  lowering dump byte for byte (including the conditional `types` object of records and variants) and refuses every
  document of the refusals dump with the generator code of its row, reading them from the oracle's directory, so an
  edit to any file there reruns the generator's tests (the same pattern as the scalar-faults fixture). Directories
  match by slash-delimited prefix (`data-forms-v10/` is not it). The row's `inputs` text names the fixture.
- `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves the files of the directory select
  `rust/warrant-corpus-gen` (and the `.warrant`, being a Warrant document, also `rust/warrant-grammar`), and that
  the lookalikes select nothing.

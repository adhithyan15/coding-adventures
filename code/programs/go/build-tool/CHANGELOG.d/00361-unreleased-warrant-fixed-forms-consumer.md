## Unreleased — the oracle's fixed-forms lowering dump selects rust/warrant-corpus-gen (CV2-S10b)

- `isWarrantCorpusGenInput` and `warrantCorpusGenPaths` gain
  `code/packages/python/warrant-oracle/tests/fixtures/fixed-forms-v1/` (the oracle's synthetic
  `ir.fixed` document and its committed lowering dump). The generator's `tests/fixed_parity.rs`
  lowers the document and reproduces the dump byte for byte, reading both files from the oracle's
  directory, so an edit to either now reruns the generator's tests (the same pattern as the
  accept-blind fixture). Directories match by slash-delimited prefix (`fixed-forms-v10/` is not it).
- `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves the three files of the directory
  select exactly `rust/warrant-corpus-gen` and that lookalikes select nothing.

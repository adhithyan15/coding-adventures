## Unreleased — the oracle's tuple-forms fixture selects rust/warrant-corpus-gen (CV2-S10t-2)

- `isWarrantCorpusGenInput` and `warrantCorpusGenPaths` gain
  `code/packages/python/warrant-oracle/tests/fixtures/tuple-forms-v1/` (the oracle's synthetic
  `ir.tuple` document, its profile and its four committed dumps: lowering, evaluation, faults and
  refusals). The generator's `tests/tuple_parity.rs` reproduces the dumps byte for byte, reading
  them from the oracle's directory, so an edit to any file there now reruns the generator's tests
  (the same pattern as the accept-blind, fixed-forms and omit-example fixtures). Directories match
  by slash-delimited prefix (`tuple-forms-v10/` is not it). The row's `inputs` text names the new
  fixture.
- `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves the files of the directory select
  `rust/warrant-corpus-gen` (and `tuple_forms_v1.warrant`, being a Warrant document, also
  `rust/warrant-grammar`, per `isWarrantDocument`), and that the lookalikes select nothing.

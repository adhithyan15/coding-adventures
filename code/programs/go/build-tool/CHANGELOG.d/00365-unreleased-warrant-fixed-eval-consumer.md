## Unreleased — the oracle's fixed-eval fixture selects rust/warrant-corpus-gen (CV2-S10c-2)

- `isWarrantCorpusGenInput` and `warrantCorpusGenPaths` gain
  `code/packages/python/warrant-oracle/tests/fixtures/fixed-eval-v1/` (the oracle's synthetic
  `ir.fixed` evaluation document, its profile and its five committed dumps: lowering, evaluation,
  faults, values and domains). The generator's `tests/fixed_eval_parity.rs` reproduces the dumps
  byte for byte, reading them from the oracle's directory, so an edit to any file there now reruns
  the generator's tests (the same pattern as the accept-blind, fixed-forms, omit-example and
  tuple-forms fixtures). Directories match by slash-delimited prefix (`fixed-eval-v10/` is not it).
  The row's `inputs` text names the new fixture.
- `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves the files of the directory select
  `rust/warrant-corpus-gen` (and `fixed_eval_v1.warrant`, being a Warrant document, also
  `rust/warrant-grammar`, per `isWarrantDocument`), and that the lookalikes select nothing.

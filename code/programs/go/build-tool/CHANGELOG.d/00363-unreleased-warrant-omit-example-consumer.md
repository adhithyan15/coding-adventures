## Unreleased — the oracle's omit-example fixture selects rust/warrant-corpus-gen (CV2-S10-1)

- `isWarrantCorpusGenInput` and `warrantCorpusGenPaths` gain
  `code/packages/python/warrant-oracle/tests/fixtures/omit-example-v2/` (the oracle's synthetic
  `omit_example` document, its profile and its committed `corpus/`). The generator's
  `tests/omit_example.rs` regenerates `corpus/` from `omit.warrant` and `corpus.profile` byte for
  byte, reading them from the oracle's directory, so an edit to any file there now reruns the
  generator's tests (the same pattern as the accept-blind and fixed-forms fixtures). Directories
  match by slash-delimited prefix (`omit-example-v20/` is not it). The row's `inputs` text names
  the new fixture.
- `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle` proves the seven files of the directory
  select `rust/warrant-corpus-gen` (and `omit.warrant`, being a Warrant document, also
  `rust/warrant-grammar`, per `isWarrantDocument`), and that the lookalikes select nothing.

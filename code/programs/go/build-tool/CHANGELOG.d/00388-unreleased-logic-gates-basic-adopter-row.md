## Unreleased — `logic-gates-basic` joins `warrantAdopters`, bindings optional (LGE-S1c)

- `warrantAdopter.bindings` is optional: an empty value is left out of `warrantAdopterBindings`,
  `warrantAdopterInputs` and `isWarrantCorpusGenInput`. The new `logic-gates-basic` row (spec
  `logic-gates-basic.warrant`, fixture `logic-gates-basic-warrant-v2`) has none, so an edit to the spec or any
  fixture file selects `rust/warrant-corpus-gen` and `python/warrant-oracle`.
- `warrant_corpus_v2_inputs_test.go` now expects 5 rows, rejects an empty path in any input list and pins
  lookalike paths to no selection.

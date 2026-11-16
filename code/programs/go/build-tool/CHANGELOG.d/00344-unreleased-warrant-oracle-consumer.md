## Unreleased — Warrant oracle repository inputs (VER04r S4)

- One repository input consumer, `python/warrant-oracle` (the independent
  `.warrant` source reader and corpus oracle). It re-derives a corpus from the
  adopter specifications, so it is selected by an edit to any of
  `ct-compare.warrant`, `gf256.warrant`, `wasm-leb128.warrant`,
  `logic-gates.warrant`, `lzw.warrant`, `arithmetic.warrant` or `x25519.warrant`
  under `code/specs/`, or to the v1 `ct-compare-warrant-v1/corpus.json` it gates.
  Exact paths only; a lookalike path does not select it.
- `TestWarrantLaneArtifactsSelectGrammarAndAssuranceConsumers` and
  `TestPythonWarrantLaneInputsSelectTheirConsumers` list the new package (a
  missing consumer fails closed) and `TestWarrantOracleInputsSelectOnlyTheOracle`
  pins the selection and the lookalike paths.

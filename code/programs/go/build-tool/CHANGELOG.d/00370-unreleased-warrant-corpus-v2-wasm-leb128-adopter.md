## Unreleased — wasm-leb128 joins the warrant-corpus-v2 adopter table (CV2 S10, VER04r 16.10 D32)

- Added the `wasm-leb128` row to `warrantAdopters` (`code/specs/wasm-leb128.warrant`,
  `code/specs/fixtures/wasm-leb128-warrant-v2`, `code/packages/rust/wasm-leb128/evidence.bindings`). An edit to the
  wasm-leb128 spec, to a file of its fixture directory or to its bindings now selects `rust/warrant-corpus-gen` and
  `python/warrant-oracle`. Selection of other rows is unchanged: `code/specs/wasm-leb128.warrant` was already an
  oracle input and a Warrant-grammar input (so it still selects those consumers; `TestGameClockInputs...` still
  holds, its package list has no generator), and now also selects the generator, whose `tests/census.rs`
  already `include_str!`s the spec (until this row an edit to that spec did not select it).
- Extended `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle`: every kind of wasm-leb128 path (fixture files
  including the largest chunk file, bindings, spec), per-language filters, wasm-leb128 lookalikes (including the
  generated evidence harness, which is not an input of either tool), and the table check now expects four rows.
  All ct-compare, x25519 and lzw assertions are kept.

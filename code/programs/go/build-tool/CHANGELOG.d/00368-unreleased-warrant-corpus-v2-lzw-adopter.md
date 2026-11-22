## Unreleased — lzw joins the warrant-corpus-v2 adopter table (CV2 S10, VER04r 16.10 D32)

- Added the `lzw` row to `warrantAdopters` (`code/specs/lzw.warrant`, `code/specs/fixtures/lzw-warrant-v2`,
  `code/packages/rust/lzw/evidence.bindings`). An edit to the lzw spec, to a file of its fixture directory or to
  its bindings now selects `rust/warrant-corpus-gen` and `python/warrant-oracle`. Selection of other rows is
  unchanged: `code/specs/lzw.warrant` was already an oracle input and a Warrant-grammar input (so it still selects
  those consumers), and now also selects the generator; the bindings file was an input of no other row.
- Extended `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle`: every kind of lzw path (fixture files including
  the largest chunk file, bindings, spec), per-language filters, lzw lookalikes, and the table check now expects
  three rows. All ct-compare and x25519 assertions are kept.

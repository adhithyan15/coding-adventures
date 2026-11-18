## Unreleased — Table-driven warrant-corpus-v2 adopter rows (VER04r 16.10 D32)

- Replaced the hard-coded ct-compare `warrantCorpusV2Fixture` / `warrantCorpusV2Bindings` constants with a
  `warrantAdopters` table of `{module, spec, fixture, bindings}` entries (ct-compare and x25519). The
  `rust/warrant-corpus-gen` and `python/warrant-oracle` rows, their declared paths and the predicates
  (`isWarrantCorpusGenInput`, `isWarrantOracleInput`, `isWarrantCorpusV2Fixture`) are built from the table, so an
  edit to an adopter's spec, to a file of its fixture directory (by slash-delimited prefix) or to its
  `evidence.bindings` selects both packages. ct-compare's selection is unchanged. `code/specs/x25519.warrant` was
  already an oracle input; it now selects the generator too.
- Extended `TestWarrantCorpusV2FixtureSelectsGeneratorAndOracle`: every kind of x25519 path (fixture files, bindings,
  spec), per-language filters, x25519 lookalikes, and a table check (distinct modules, every row selected by both
  predicates). Every existing ct-compare assertion is kept.

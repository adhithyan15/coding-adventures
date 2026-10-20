# Changelog

## Unreleased

- LANG84 phase 4c: regenerated for the corrected `perl5.38.2`, `perl5.44.0`
  and `perl5.45.3` pairs. `perl5.38.2` is now a copy of `perl5.38.0` (same
  syntax blobs), so it joins that token table, and `perl5.44.0`/`perl5.45.3`
  bound `INT` at 1018 digits, so their table is replaced by a bounded one.
  Still 118 checked releases and 5 distinct token tables.

- LANG84 phase 2b: regenerated after the `releases.csv` schema migration.
  The rows are now in date order, so `INVENTORY_RELEASES`,
  `SUPPORTED_RELEASES` and the match arms list releases chronologically
  (1.0.15 and 1.0_16 now come after the 2002 releases, for example). The
  token tables, the `g_` modules and the first release of each class are
  unchanged.

- LANG84 phase 4b: regenerated for the new `perl5.38.0` partial pair
  (118 checked releases, still 5 distinct token tables: 5.38.0 shares
  5.004_72's). The unversioned table remains for `tokenize_perl` only.

- LANG84 phase 4a: compile every checked release pair. `_grammar.rs` is now
  generated from `releases.csv` (117 pairs, 5 distinct tables) and exports
  `INVENTORY_RELEASES`, `SUPPORTED_RELEASES`, `release_token_grammar(id)`
  (`None` for pending or unknown IDs) and `unversioned_token_grammar()`.
  `tokenize_perl` still uses the unversioned LANG81 grammar. New tests pin the
  compiled tables against the inventory and against each pair parsed fresh.

- Limit the pilot integer token to plain decimal forms; generated Rust token
  data and parser probes now reject leading-zero multi-digit forms before
  InterpreterIR lowering.

## 0.1.0

- Initial bounded Perl-language lexer for LANG81.

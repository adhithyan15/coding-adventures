# Changelog

## Unreleased

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

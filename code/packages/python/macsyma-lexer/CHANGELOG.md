# Changelog

## 0.1.0 — 2026-04-19

Initial release.

- Thin wrapper around `GrammarLexer`, configured via
  `code/grammars/macsyma/macsyma.tokens`.
- Supports integer/float/scientific numbers, names (including
  `%`-prefixed constants), strings, all MACSYMA operators (including
  `:=`, `:`, `=`, `#`, `<=`, `>=`, `->`, `**`), delimiters, two
  statement terminators (`;` and `$`), C-style comments.
- Keywords: `and`, `or`, `not`, `true`, `false`, plus control-flow
  words (`if`, `then`, `else`, `for`, `while`, etc.) carried through
  for future parser extensions.
- Full test suite covering every token category and realistic
  MACSYMA programs.

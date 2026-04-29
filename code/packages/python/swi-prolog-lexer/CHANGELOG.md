# Changelog

## Unreleased

- Tokenize SWI CLP(FD) range syntax (`..`) as one symbolic atom so
  finite-domain expressions such as `1..4` can parse as operator terms.

## 0.1.0

- Added the first SWI-Prolog lexer package backed by `code/grammars/prolog/swi.tokens`

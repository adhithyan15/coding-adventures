# Changelog

## 0.1.1 — 2026-08-17

### Fixed

- Eliminated runtime grammar loading: `create_lisp_lexer` now imports a
  pre-compiled `_grammar` module instead of reading and parsing the
  `.tokens` file from `code/grammars/` on every call. The old code walked
  out of the installed package's own directory to a monorepo-relative path
  that a published PyPI package does not ship, so `pip install` + first use
  would raise `FileNotFoundError`. The previous `_grammar.py` in this
  package was a stray, unwired artifact never imported by `tokenizer.py`;
  it has been regenerated fresh from `code/grammars/lisp/lisp.tokens` and
  is now actually wired in.

## 0.1.0 — 2026-03-20

### Added

- **Lisp lexer** — thin wrapper around grammar-tools GrammarLexer
- Loads `lisp.tokens` grammar file for token definitions
- `create_lisp_lexer()` factory and `tokenize_lisp()` convenience function
- Tokens: NUMBER, SYMBOL, STRING, LPAREN, RPAREN, QUOTE, DOT
- Skips WHITESPACE and COMMENT tokens
- 33 tests, 100% coverage

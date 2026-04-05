# Changelog — TypeScript 3.0 (2018) Lexer

## 0.1.1 (2026-04-05)

### Fixed

- Moved `enum`, `async`, `await` from `reserved:`/`context_keywords:` to
  `keywords:` in ts3.0.tokens so they emit as `KEYWORD` tokens consistently
  with ES2017+ semantics

## 0.1.0 (2026-04-05)

### Added

- Initial implementation of the TypeScript 3.0 (2018) lexer
- Thin wrapper around `GrammarLexer` loading `ts3.0.tokens`
- Public API: `create_ts30_lexer(source)`, `tokenize_ts30(source)`
- Comprehensive test suite covering keywords, type annotations, generics, operators
- Tests for TS 3.0-specific features: `unknown` type, tuple rest elements

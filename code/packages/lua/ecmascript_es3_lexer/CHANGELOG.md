# Changelog -- coding-adventures-ecmascript-es3-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] -- 2026-04-05

### Added

- Initial implementation of `coding_adventures.ecmascript_es3_lexer`.
- `tokenize(source)` -- tokenizes an ECMAScript 3 string using the shared
  `ecmascript/es3.tokens` grammar and the grammar-driven `GrammarLexer`.
- `get_grammar()` -- returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/ecmascript/es3.tokens` once and cached.
- Full ES3 token set including strict equality (`===`, `!==`), error handling
  keywords (`try`, `catch`, `finally`, `throw`), and `instanceof`.
- Comprehensive busted test suite.
- `BUILD` and `BUILD_windows` scripts.

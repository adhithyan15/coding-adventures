# Changelog -- coding-adventures-ecmascript-es5-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] -- 2026-04-05

### Added

- Initial implementation of `coding_adventures.ecmascript_es5_lexer`.
- `tokenize(source)` -- tokenizes an ECMAScript 5 string using the shared
  `ecmascript/es5.tokens` grammar and the grammar-driven `GrammarLexer`.
- `get_grammar()` -- returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/ecmascript/es5.tokens` once and cached.
- Full ES5 token set including `debugger` keyword and all ES3 features.
- Comprehensive busted test suite.
- `BUILD` and `BUILD_windows` scripts.

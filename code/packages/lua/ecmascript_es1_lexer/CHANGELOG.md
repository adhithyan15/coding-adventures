# Changelog -- coding-adventures-ecmascript-es1-lexer (Lua)

All notable changes to this package are documented here.

## [0.1.0] -- 2026-04-05

### Added

- Initial implementation of `coding_adventures.ecmascript_es1_lexer`.
- `tokenize(source)` -- tokenizes an ECMAScript 1 string using the shared
  `ecmascript/es1.tokens` grammar and the grammar-driven `GrammarLexer` from
  `coding-adventures-lexer`.
- `get_grammar()` -- returns the cached `TokenGrammar` for direct use.
- Grammar is read from `code/grammars/ecmascript/es1.tokens` once and cached.
- Path navigation uses `debug.getinfo` to locate the grammar file relative
  to the installed module, avoiding hardcoded absolute paths.
- Full ES1 token set: NAME, NUMBER, STRING; 23 keyword tokens; all ES1
  operators including compound assignments, shifts, and bitwise operators;
  all delimiter tokens.
- Comprehensive busted test suite covering keywords, identifiers, numbers,
  strings, operators, delimiters, composite expressions, whitespace handling,
  position tracking, and error cases.
- `required_capabilities.json` declaring `filesystem:read`.
- `BUILD` and `BUILD_windows` scripts with transitive dependency
  installation in leaf-to-root order.

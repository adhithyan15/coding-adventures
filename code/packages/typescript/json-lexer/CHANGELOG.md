# Changelog

All notable changes to the JSON Lexer package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `tokenizeJSON` now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `json.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`. The previously-committed `_grammar.ts` (unused until now) was also stale — missing an additive `softKeywords: []` field — and has been regenerated.

## [0.1.0] - 2026-03-20

### Added

- Initial release of the JSON lexer.
- `tokenizeJSON()` function that tokenizes JSON text using the grammar-driven lexer engine.
- Loads `json.tokens` grammar file defining STRING, NUMBER, TRUE, FALSE, NULL, and structural tokens.
- Full support for JSON number formats: integers, negatives, decimals, scientific notation.
- Full support for JSON string escape sequences: `\"`, `\\`, `\/`, `\b`, `\f`, `\n`, `\r`, `\t`, `\uXXXX`.
- Comprehensive test suite covering all JSON token types, nested structures, whitespace handling, position tracking, and error cases.

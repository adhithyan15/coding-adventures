# Changelog

All notable changes to the Ruby Lexer (TypeScript) package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `tokenizeRuby` now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `ruby.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Ruby lexer package.
- `tokenizeRuby()` function that tokenizes Ruby source code using the grammar-driven lexer.
- Loads `ruby.tokens` grammar file from `code/grammars/`.
- Supports Ruby keywords, operators (`..`, `=>`, `!=`, `<=`, `>=`), strings, and numbers.
- Comprehensive test suite with v8 coverage.

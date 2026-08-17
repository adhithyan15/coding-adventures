# Changelog

## [0.1.0] - Unreleased

### Added

- Added a TypeScript Lisp lexer wrapper over the shared grammar-driven lexer.
- Added tests for lists, symbols, comments, strings, dotted pairs, factory creation, and EOF handling.

### Fixed

- Eliminated runtime grammar loading: now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `lisp.tokens` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

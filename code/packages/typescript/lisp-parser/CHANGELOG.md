# Changelog

## [0.1.0] - Unreleased

### Fixed

- Eliminated runtime grammar loading: now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `lisp.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

### Added

- Added a TypeScript Lisp parser wrapper over the shared grammar-driven parser.
- Added tests for atoms, lists, quoted forms, dotted pairs, parser creation, and parse errors.

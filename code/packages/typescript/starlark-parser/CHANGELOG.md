# Changelog

All notable changes to the Starlark Parser (TypeScript) package will be documented in this file.

## [0.1.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parseStarlark` now imports a pre-compiled `_grammar.ts` instead of `readFileSync`-ing `starlark.grammar` from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript Starlark parser package.
- `parseStarlark()` function that parses Starlark source code into generic `ASTNode` trees.
- Loads `starlark.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/starlark-lexer`.
- Supports assignments (simple and augmented), function definitions, if/elif/else, for loops, load statements, BUILD-file style function calls with named arguments, list/dict literals, comprehensions, and full operator precedence.
- Comprehensive test suite with v8 coverage.

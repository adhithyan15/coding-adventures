# Changelog

All notable changes to the JavaScript Parser (TypeScript) package will be documented in this file.

## [0.2.1] - 2026-08-17

### Fixed
- Eliminated runtime grammar loading: `parseJavascript`/`createJavascriptParser` now import a pre-compiled `_grammar[_es<version>].ts` per ECMAScript edition instead of `readFileSync`-ing the `.grammar` file from `code/grammars/` on every call. The old code walked out of the installed package's own directory to a monorepo-relative path that a published npm package does not ship, so `npm install` + first use would throw `ENOENT`.
- Compiled `es2018.grammar` through `es2025.grammar` (8 files) with the new `--force` flag (see csharp-parser's changelog): each has a pre-existing `async_generator_expression`-unreachable validation warning (unlike its sibling `async_generator_declaration`, it's never wired into `primary_expression`) that otherwise blocks compilation.

## [0.2.0] - 2026-04-05

### Added
- `parseJavascript(source, version?)` — optional `version` parameter accepting
  `"es1"`, `"es3"`, `"es5"`, `"es2015"` through `"es2025"`.
  When omitted (or empty string), the generic grammars are used — backwards-compatible
  with v0.1.x.
- Versioned grammar support loads parser grammar from `code/grammars/ecmascript/<version>.grammar`
  and delegates to the versioned lexer grammar automatically.
- Clear error thrown for unrecognised version strings.
- Expanded test suite covering all supported ES version strings, empty-string version,
  and error cases.

### Changed
- `parseJavascript` signature is now `(source: string, version?: string): ASTNode`
  — fully backwards-compatible; existing callers with one argument are unaffected.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript JavaScript parser package.
- `parseJavascript()` function that parses JavaScript source code into generic `ASTNode` trees.
- Loads `javascript.grammar` file from `code/grammars/`.
- Delegates tokenization to `@coding-adventures/javascript-lexer`.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with v8 coverage.

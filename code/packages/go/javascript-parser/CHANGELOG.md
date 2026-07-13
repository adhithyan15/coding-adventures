# Changelog

## [0.3.0] - 2026-07-13

### Changed

- The parser grammar is now embedded at compile time as native Go data in
  `grammar_data.go` and consumed directly by the loader; the package no longer
  reads `code/grammars/**` at run time via `runtime.Caller`. This drops the
  filesystem capability (empty `required_capabilities.json`, `gen_capabilities.go`
  removed), lets the package build and run standalone, and fixes the previously
  dead embedded grammar (the old `_grammar.go` name is ignored by the Go
  toolchain because of its leading underscore).

## [0.2.0] - 2026-04-05

### Added
- `version string` parameter added to `ParseJavascript`, `CreateJavascriptParser`,
  and the internal `getGrammarPath` helper.
- Valid versions: `"es1"`, `"es3"`, `"es5"`, `"es2015"`, `"es2016"`, `"es2017"`,
  `"es2018"`, `"es2019"`, `"es2020"`, `"es2021"`, `"es2022"`, `"es2023"`,
  `"es2024"`, `"es2025"`.
- Empty string `""` preserves the pre-0.2.0 behaviour (uses the generic
  `javascript.grammar`), keeping existing callers backward-compatible.
- Unknown version strings return a descriptive error rather than silently
  falling back to the generic grammar.
- `required_capabilities.json` updated to declare all 15 allowed grammar file
  paths (1 generic + 14 versioned).
- `gen_capabilities.go` updated with 15 `_allowedPath_N` vars and updated
  `ReadFile` capability guard to match any of the 15 allowed paths.
- 15 new tests covering each versioned grammar and the error path for unknown
  versions.

## [0.1.1] - 2026-03-31

### Fixed

- Fixed parse failures for `let`/`const` variable declarations by relying on
  the shared fix to `javascript.grammar`. The grammar now uses `KEYWORD` instead
  of `VAR | LET | CONST`, matching what the lexer actually emits.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Go JavaScript parser package.
- `ParseJavascript()` function that parses JavaScript source code into generic `ASTNode` trees.
- `CreateJavascriptParser()` factory function.
- Loads `javascript.grammar` from `code/grammars/`.

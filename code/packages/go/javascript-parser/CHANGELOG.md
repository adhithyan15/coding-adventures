# Changelog

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

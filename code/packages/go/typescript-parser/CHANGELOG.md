# Changelog

## [0.1.1] - 2026-03-31

### Fixed

- Fixed parse failures for `let`/`const` variable declarations by relying on
  the shared fix to `typescript.grammar`. The grammar now uses `KEYWORD` instead
  of `VAR | LET | CONST`, matching what the lexer actually emits.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the Go TypeScript parser package.
- `ParseTypescript()` function that parses TypeScript source code into generic `ASTNode` trees.
- `CreateTypescriptParser()` factory function.
- Loads `typescript.grammar` from `code/grammars/`.

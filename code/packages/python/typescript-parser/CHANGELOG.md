# Changelog

All notable changes to the TypeScript Parser package will be documented in this file.

## [0.2.0] - 2026-04-05

### Added
- `version` parameter added to `parse_typescript()` and `create_typescript_parser()`.
  Pass `"ts1.0"` through `"ts5.8"` to load the corresponding versioned grammar files
  (both `.tokens` and `.grammar`).  Omitting `version` continues to use the generic
  grammars — backward compatible.
- `_resolve_grammar_path(version)` private helper alongside the existing `_GRAMMAR_ROOT`
  and `_VALID_VERSIONS` constants, mirroring the lexer's design.
- `version` is forwarded to `tokenize_typescript()` so the lexer and parser always
  use the same versioned grammar.
- Raises `ValueError` with a clear message for unknown version strings.
- Version-specific tests covering all six supported versions plus error handling.

## [0.1.1] - 2026-03-31

### Fixed

- Updated `typescript.grammar` so that `var_declaration` and `factor` reference
  `KEYWORD` instead of `VAR | LET | CONST` and `TRUE | FALSE | NULL | UNDEFINED`.
  The grammar-driven lexer reclassifies all keyword identifiers to `KEYWORD` tokens.
  This fixes `GrammarParseError: Parse error at 1:1: Unexpected token: 'let'`.

## [0.1.0] - 2026-03-19

### Added
- Initial release of the TypeScript parser package.
- `parse_typescript()` function that parses TypeScript source code into generic `ASTNode` trees.
- `create_typescript_parser()` factory function for creating a `GrammarParser` configured for TypeScript.
- Supports `var_declaration` (let/const/var), assignments, expression statements, and operator precedence.
- Comprehensive test suite with 80%+ coverage.

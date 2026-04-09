# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- Initial release of the ALGOL 60 parser crate.
- `create_algol_parser()` factory function returning a `GrammarParser` configured for ALGOL 60.
- `parse_algol()` convenience function returning `GrammarASTNode` directly.
- Loads the `algol.grammar` file at runtime from the shared `grammars/` directory.
- Full ALGOL 60 grammar support: program, block, declarations (type, array, switch, procedure), statements (assign, conditional, for, goto, proc call, compound, empty), expressions (arithmetic with operator precedence, boolean with eqv/impl/or/and/not, designational).
- Depends on `coding-adventures-algol-lexer` for tokenization.
- 16 unit tests covering: minimal program, block structure, assignment, arithmetic expression, if/then, if/then/else, for loop (step/until form), type declaration, real declaration, factory function, compound statement, exponentiation (`**` and `^`), boolean expressions, goto, procedure call, and for loop (while form).

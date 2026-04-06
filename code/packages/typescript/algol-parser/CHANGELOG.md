# Changelog — @coding-adventures/algol-parser

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This package uses [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-06

### Added

- Initial release of the ALGOL 60 parser.
- `parseAlgol(source: string): ASTNode` — parses ALGOL 60 source text into an AST using the grammar-driven `GrammarParser` engine loaded with `algol.grammar`.
- Full ALGOL 60 grammar support: programs, blocks, declarations (type, array, switch, procedure), statements (assign, goto, procedure call, compound, for, conditional), and expressions (arithmetic, boolean, designational).
- Integrates with `@coding-adventures/algol-lexer` for the tokenization stage.
- Top-level AST node is `"program"`, wrapping a `"block"` node.
- Comprehensive test suite covering: minimal programs, integer/real/boolean declarations, assignment, arithmetic (addition, subtraction, multiplication, division, compound expressions, parenthesized expressions), conditionals (if/then, if/then/else, chained if/else if), for loops (step/until form, compound body), boolean expressions (equality, relational, and/or/not), comment skipping, and nested blocks.

# Changelog

All notable changes to the `@coding-adventures/parser` package will be documented in this file.

## [0.3.0] - 2026-04-18

### Added

- Optional rich source preservation for `GrammarParser` via
  `{ preserveSourceInfo: true }`.
- Optional AST node metadata fields:
  - `startOffset` / `endOffset`
  - `firstTokenIndex` / `lastTokenIndex`
  - `leadingTrivia`
- Cached grammar-parser nodes now rebuild token-derived metadata instead of
  dropping it on memoized parses.

## [0.2.0] - 2026-03-23

### Added

- `GrammarParserOptions` interface exported from `src/grammar-parser.ts` and `src/index.ts`.
- Trace mode in `GrammarParser`: pass `{ trace: true }` as the third constructor argument to enable per-rule trace output on `process.stderr`.
  - Each rule attempt writes a `[TRACE]` line of the form:
    `[TRACE] rule '<name>' at token <index> (<TYPE> "<value>") → match|fail`
  - Output goes to stderr so it does not interfere with any structured stdout output.
  - Trace is written only for non-cached rule attempts; cached hits are transparent.
  - With `trace: false` or no options, no stderr output is produced (default behavior unchanged).
- 8 new trace-mode tests in `tests/grammar-parser.test.ts`:
  - Correct parse result when trace is on.
  - `[TRACE]` lines appear on stderr.
  - Each trace line matches the canonical format (validated via regex).
  - Both `→ match` and `→ fail` outcomes appear for non-trivial input.
  - No stderr output when trace is disabled (default and explicit `false`).
  - Token identity (type + value) is present in trace output.
  - Assignment input works correctly with trace enabled.

### Fixed

- Added `@coding-adventures/state-machine` as an explicit dependency to `package.json` so that the transitive dependency from `@coding-adventures/lexer` resolves correctly during `npm install`.

## [0.1.0] - 2026-03-19

### Added

- **Hand-written recursive descent parser** (`Parser` class)
  - Parses token streams into typed AST nodes (`NumberLiteral`, `StringLiteral`, `Name`, `BinaryOp`, `Assignment`, `Program`)
  - Implements operator precedence via grammar rule nesting (multiplication/division before addition/subtraction)
  - Left-associative operator parsing
  - Parenthesized expression support for precedence override
  - Assignment statement parsing with LL(2) lookahead
  - Expression statement parsing
  - `ParseError` class with token location information

- **Grammar-driven parser** (`GrammarParser` class)
  - Interprets EBNF grammar rules from `.grammar` files at runtime
  - Produces generic `ASTNode` trees (language-agnostic)
  - Supports all EBNF constructs: Sequence, Alternation, Repetition, Optional, Group, TokenReference, RuleReference, Literal
  - Backtracking for alternation handling
  - Automatic newline skipping within expressions
  - `GrammarParseError` class with optional token location
  - Helper functions: `isASTNode()`, `isLeafNode()`, `getLeafToken()`

- **Full test suite** with >80% coverage
  - Hand-written parser tests: atoms, binary ops, precedence, parentheses, assignments, multiple statements, error handling, end-to-end
  - Grammar-driven parser tests: same coverage using actual `python.grammar` file

- Knuth-style literate programming throughout all source files
- TypeScript port from Python `lang_parser` package

# Changelog — twig-parser

## [0.1.0] — 2026-04-29

### Added

- Initial Rust implementation of the Twig parser (TW00).
- Thin wrapper around the generic [`parser::grammar_parser::GrammarParser`](../parser),
  driven by `code/grammars/twig.grammar` — the canonical Twig parser
  grammar shared with the Python implementation.
- Public entries:
  - `parse(source) -> Result<Program, TwigParseError>` — lex + grammar-parse
    + extract typed AST in one call.
  - `parse_to_ast(source) -> Result<GrammarASTNode, TwigParseError>` —
    stop at the generic AST tree.
  - `create_twig_parser(source) -> GrammarParser` — for callers that
    want the parser object (tracing, alternative entry rules).
  - `create_twig_parser_from_tokens(tokens) -> GrammarParser` — pre-tokenised
    input for LSP-style flows.
- Typed AST: `Program`, `Form`, `Define`, `Expr` (with `IntLit`,
  `BoolLit`, `NilLit`, `SymLit`, `VarRef`, `If`, `Let`, `Begin`,
  `Lambda`, `Apply` variants).
- `ast_extract` module walks the generic `GrammarASTNode` tree → typed
  AST.  Mirrors the Python package's `ast_extract.py`.
- Define-sugar lowering at extraction time: `(define (f x) body+)` →
  `Define { name: "f", expr: Lambda { params: ["x"], body } }`.
- Both quote forms (`'foo` and `(quote foo)`) collapse to a single
  `SymLit { name: "foo" }`.
- Source-position tracking on every AST node (1-indexed `line` /
  `column`), propagated from the underlying tokens.
- `TwigParseError { message, line, column }` with
  `From<GrammarParseError>` so grammar errors propagate transparently.
- **Stack-overflow defence** — `MAX_PAREN_DEPTH = 64` cap pre-scans
  the token stream and rejects deeply-nested untrusted input before
  invoking the recursive `GrammarParser`.  Without this cap a
  pathological source like `(((...)))` with thousands of opens would
  abort the process via OS thread stack-overflow (Rust does not catch
  stack overflow).
- `MAX_AST_DEPTH = 256` cap in the extractor bounds recursion when
  callers bypass `parse()` and feed in a hand-built AST.
- 31 unit tests covering every form, sugar lowering, position
  tracking, depth cap, and error paths.

# Changelog — twig-parser

## [0.1.0] — 2026-04-29

### Added

- Initial Rust implementation of the Twig parser (TW00).
- Recursive-descent parser with one method per grammar non-terminal.
- Typed AST: `Program`, `Form`, `Define`, `Expr` (with `IntLit`,
  `BoolLit`, `NilLit`, `SymLit`, `VarRef`, `If`, `Let`, `Begin`,
  `Lambda`, `Apply` variants).
- `parse(source)` — lex + parse Twig source in one call.
- `parse_tokens(tokens)` — parse a pre-tokenised stream.
- Define-sugar lowering at parse time: `(define (f x) body+)` →
  `Define { name: "f", expr: Lambda { params: ["x"], body } }`.
- Both quote forms (`'foo` and `(quote foo)`) collapse to a single
  `SymLit { name: "foo" }` AST node.
- Source-position tracking on every AST node (1-indexed `line` /
  `column`), inherited from `twig-lexer` tokens.
- `TwigParseError { message, line, column }` with `From<LexerError>`
  conversion so lexer errors propagate transparently.
- Strict shape validation: `(if ...)` requires exactly 3 expressions,
  `(define name expr)` requires exactly one body, `(let ((..)) body+)`
  / `(begin body+)` / `(lambda () body+)` all require at least one
  body expression, empty `()` is a hard error.
- Nested `(define ...)` is rejected at parse time with a clear message
  ("only allowed at the top level") so users don't get a confusing
  compile-time error later.
- 30 unit tests covering atoms, quotes, applies, `if` / `let` /
  `begin` / `lambda` / `define`, sugar lowering, position tracking,
  and error paths.

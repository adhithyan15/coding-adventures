# Changelog — coding-adventures-excel-parser

## 0.1.0 — 2026-03-29

### Added
- Initial implementation of `coding_adventures.excel_parser`.
- Hand-written recursive-descent parser for Excel formulas.
- Full operator precedence hierarchy: comparison < concat(&) < additive <
  multiplicative < power < unary < postfix(%).
- AST node kinds: `formula`, `binop`, `unop`, `postfix`, `call`, `range`,
  `ref_prefix`, `cell`, `number`, `string`, `bool`, `error`, `name`,
  `array`, `group`.
- Parses all arithmetic, comparison, and concatenation operators.
- Parses function calls with arbitrary argument lists including empty args.
- Parses range references (A1:B10) and cross-sheet references (Sheet1!A1).
- Parses array constants ({1,2;3,4}).
- Delegates tokenization to `coding-adventures-excel-lexer`.
- Exposes `M.parse(source)` and `M.tokenize(source)`.
- Comprehensive busted test suite covering all grammar productions.
- `required_capabilities.json` declaring `filesystem:read` (transitively).
- `CHANGELOG.md`, `README.md`, and `BUILD` file.

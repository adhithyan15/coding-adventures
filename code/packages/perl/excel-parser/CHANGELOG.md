# Changelog — CodingAdventures::ExcelParser

## 0.01 — 2026-03-29

### Added
- Initial implementation of `CodingAdventures::ExcelParser`.
- Hand-written recursive-descent parser for Excel formula strings.
- Full operator precedence: comparison < concat(&) < additive <
  multiplicative < power < unary < postfix(%).
- AST node kinds (via `CodingAdventures::ExcelParser::ASTNode`):
  `formula`, `binop`, `unop`, `postfix`, `call`, `range`, `ref_prefix`,
  `cell`, `number`, `string`, `bool`, `error`, `name`, `array`, `group`.
- Parses all arithmetic, comparison, and concatenation operators.
- Parses function calls with arbitrary argument lists (including empty args).
- Parses range references (A1:B10) and cross-sheet references (Sheet1!A1).
- Parses array constants ({1,2;3,4}).
- Delegates tokenization to `CodingAdventures::ExcelLexer`.
- `CodingAdventures::ExcelParser::ASTNode` submodule for AST nodes.
- `t/00-load.t` — module load test.
- `t/01-basic.t` — comprehensive parser tests covering all grammar productions.
- `Makefile.PL`, `cpanfile`, `required_capabilities.json`,
  `CHANGELOG.md`, `README.md`, `BUILD`.

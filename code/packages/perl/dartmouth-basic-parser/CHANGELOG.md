# Changelog — CodingAdventures::DartmouthBasicParser (Perl)

All notable changes to this package will be documented in this file.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.01] — 2026-04-10

### Added

- Initial implementation of the hand-written recursive-descent Dartmouth
  BASIC parser.
- `CodingAdventures::DartmouthBasicParser->parse($source)` — tokenizes
  BASIC source via `CodingAdventures::DartmouthBasicLexer`, then parses
  the token stream into an AST. Returns the root ASTNode with
  `rule_name == "program"`. Dies on lexer or parse errors.
- Full implementation of all 17 BASIC statement types:
  - `_parse_let_stmt` — `LET variable EQ expr`
  - `_parse_print_stmt` — `PRINT [ print_list ]`
  - `_parse_input_stmt` — `INPUT variable { COMMA variable }`
  - `_parse_if_stmt` — `IF expr relop expr THEN LINE_NUM`
  - `_parse_goto_stmt` — `GOTO LINE_NUM`
  - `_parse_gosub_stmt` — `GOSUB LINE_NUM`
  - `_parse_return_stmt` — `RETURN`
  - `_parse_for_stmt` — `FOR NAME EQ expr TO expr [ STEP expr ]`
  - `_parse_next_stmt` — `NEXT NAME`
  - `_parse_end_stmt` — `END`
  - `_parse_stop_stmt` — `STOP`
  - `_parse_rem_stmt` — `REM` (comment body already removed by lexer)
  - `_parse_read_stmt` — `READ variable { COMMA variable }`
  - `_parse_data_stmt` — `DATA NUMBER { COMMA NUMBER }`
  - `_parse_restore_stmt` — `RESTORE`
  - `_parse_dim_stmt` — `DIM NAME(NUMBER) { COMMA NAME(NUMBER) }`
  - `_parse_def_stmt` — `DEF USER_FN LPAREN NAME RPAREN EQ expr`
- Expression precedence cascade: `_parse_expr`, `_parse_term`,
  `_parse_power` (right-associative), `_parse_unary`, `_parse_primary`.
- `_parse_variable` — handles both scalar (`NAME`) and array (`NAME(expr)`)
  forms with 2-token lookahead.
- `_parse_relop` — all 6 relational operators: EQ, LT, GT, LE, GE, NE.
- `_expect` / `_expect_keyword` / `_peek_keyword` helpers for clean token
  consumption with descriptive error messages.
- `CodingAdventures::DartmouthBasicParser::ASTNode` — lightweight
  blessed-hashref AST node with `rule_name`, `children`, `is_leaf`, `token`
  accessors.
- Test suite (`t/00-load.t`, `t/01-basic.t`):
  - All 17 BASIC statement types
  - All 6 relational operators in IF statements
  - Expression precedence (expr, term, power, unary, primary)
  - Multi-line programs (HELLO WORLD, FOR loop, GOTO loop, GOSUB/RETURN,
    READ/DATA, REM)
  - Empty program and bare line number
  - ASTNode accessor tests
  - Error cases (missing THEN, incomplete LET, incomplete FOR)

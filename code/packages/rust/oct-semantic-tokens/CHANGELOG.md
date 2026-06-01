# Changelog — `oct-semantic-tokens`

## 0.1.0 — 2026-06-01 (OCT-ST01 — initial Oct semantic tokens)

Initial release.  Sibling of `basic-semantic-tokens` 0.1.0
(PR #4717), `nib-semantic-tokens` 0.1.0 (PR #4718), and
`twig-semantic-tokens` 0.2.0.  **Completes the syntax-highlighting
phase** (task #40 — part 3 of 3).

### What's here

- `TokenKind` enum (`Keyword`, `Intrinsic`, `Boolean`, `Number`,
  `Type`, `Variable`, `Operator`, `Punctuation`, `Comment`) with
  stable LSP-aligned mnemonics.  `Intrinsic` is Oct-specific —
  Oct's 8008 hardware-intrinsic keywords (`in`, `out`, `adc`,
  `sbb`, `rlc`, `rrc`, `ral`, `rar`, `carry`, `parity`) get a
  separate kind so themes can colour them like CPU mnemonics
  rather than control flow.
- `SemanticToken` struct — `(line, column, length, kind)` in
  1-based monospace cells.
- `semantic_tokens(source) -> Vec<SemanticToken>` — tokenise + classify.
- `tokens_from(tokens) -> Vec<SemanticToken>` — reuse a lex pass.

### Token classifications

| Origin (oct-lexer)                                          | `TokenKind` |
|-------------------------------------------------------------|-------------|
| `type_name == "fn" / "let" / "static" / "if" / …`            | `Keyword`   |
| `type_name == "in" / "out" / "adc" / "sbb" / "rlc" / "rrc" / "ral" / "rar" / "carry" / "parity"` | `Intrinsic` |
| `type_name == "true" / "false"`                             | `Boolean`   |
| `type_name == "INT_LIT" / "HEX_LIT" / "BIN_LIT"`            | `Number`    |
| `NAME` matching `u8 / bool`                                  | `Type`      |
| Other `NAME`                                                 | `Variable`  |
| `PLUS / MINUS / AMP / PIPE / CARET / TILDE / BANG / LT / GT / EQ / EQ_EQ / NEQ / LEQ / GEQ / LAND / LOR` | `Operator` |
| `COMMA / SEMICOLON / COLON / LPAREN / RPAREN / LBRACE / RBRACE / ARROW` | `Punctuation` |
| `LINE_COMMENT`                                              | `Comment`   |
| `WHITESPACE / Newline / Eof`                                | (skipped)   |

### Tests

- 10 unit tests covering each classification family.

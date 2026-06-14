# Changelog — `nib-semantic-tokens`

## 0.1.0 — 2026-06-01 (NIB-ST01 — initial Nib semantic tokens)

Initial release.  Sibling of `basic-semantic-tokens` 0.1.0
(PR #4717) and `twig-semantic-tokens` 0.2.0.  Part 2 of 3 of the
syntax-highlighting phase (task #39).

### What's here

- `TokenKind` enum (`Keyword`, `Boolean`, `Number`, `Type`,
  `Variable`, `Operator`, `Punctuation`) with stable LSP-aligned
  mnemonics.
- `SemanticToken` struct — `(line, column, length, kind)` in
  1-based monospace cells.
- `semantic_tokens(source) -> Vec<SemanticToken>` — tokenise + classify.
- `tokens_from(tokens) -> Vec<SemanticToken>` — reuse a lex pass
  when callers already have it.

### Token classifications

| Origin (nib-lexer)                               | `TokenKind` |
|--------------------------------------------------|-------------|
| `type_name == "fn" / "let" / "return" / …`        | `Keyword`   |
| `type_name == "true" / "false"`                  | `Boolean`   |
| `type_name == "INT_LIT" / "HEX_LIT"`             | `Number`    |
| `NAME` matching `u4 / u8 / u16 / u32 / bool`      | `Type`      |
| Other `NAME`                                      | `Variable`  |
| `TokenType::Plus / Minus / Star / Slash / Equals` | `Operator` |
| `TokenType::Comma / Semicolon / Colon / LParen / RParen / LBrace / RBrace` | `Punctuation` |
| `TokenType::Newline / Eof / Indent / Dedent`      | (skipped)   |

### Tests

- 9 unit tests:
  - `tokens_returns_in_document_order`
  - `classifies_fn_keyword`
  - `classifies_let_keyword`
  - `classifies_true_false_as_boolean`
  - `classifies_int_and_hex_literals_as_number`
  - `classifies_type_names`
  - `classifies_user_name_as_variable`
  - `classifies_operators_and_punctuation`
  - `token_kind_mnemonic_stable`

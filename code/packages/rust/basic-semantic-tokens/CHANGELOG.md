# Changelog — `basic-semantic-tokens`

## 0.1.0 — 2026-05-30 (BASIC-ST01 — initial Dartmouth BASIC semantic tokens)

Initial release.  The BASIC counterpart to `twig-semantic-tokens`
0.2.0 — first crate in the BASIC LSP / editor stack.

### What's here

- `TokenKind` enum (`LineNumber`, `Keyword`, `Number`, `String`,
  `Variable`, `BuiltinFn`, `UserFn`, `Operator`, `Punctuation`)
  with stable string mnemonics matching LSP semantic-token-type
  conventions where they line up.
- `SemanticToken` struct — `(line, column, length, kind)` in
  1-based monospace cells.
- `semantic_tokens(source) -> Vec<SemanticToken>` — tokenise + classify.
- `tokens_from(tokens) -> Vec<SemanticToken>` — reuse the lex
  output when callers already have it.

### Token classifications

| `effective_type_name()` | `TokenKind`     | Notes |
|-------------------------|-----------------|-------|
| `LINE_NUM`              | `LineNumber`    | Custom grammar type for the leading integer on each line. |
| `KEYWORD`               | `Keyword`       | `LET`, `PRINT`, `INPUT`, `IF`, `THEN`, `GOTO`, `FOR`, `NEXT`, `END`, `REM`, etc. |
| `NUMBER`                | `Number`        | After `LINE_NUM` rewriting; standalone numeric literals. |
| `STRING`                | `String`        | `"HELLO"` style literals. |
| `NAME`                  | `Variable`      | Single-letter or letter-digit variables (`A`, `B7`, `X0`). |
| `BUILTIN_FN`            | `BuiltinFn`     | `SIN`, `COS`, `SQR`, `ABS`, `INT`, `RND`. |
| `USER_FN`               | `UserFn`        | DEF-FN-named functions (`FNA` … `FNZ9`). |
| `Plus`/`Minus`/…/`Equals`/`Less`/`Greater` | `Operator` | Arithmetic, relational, assignment. |
| `Comma`/`Semicolon`/`Colon`/`LParen`/`RParen` | `Punctuation` | Grouping and separation. |
| `Newline` / `Eof`       | (skipped)       | Editors don't highlight either. |

### Tests

- 7 unit tests:
  - `tokens_returns_in_document_order`
  - `classifies_line_number_and_keyword`
  - `classifies_string_literal`
  - `classifies_variable_and_number`
  - `classifies_builtin_fn`
  - `classifies_operator`
  - `empty_source_returns_empty_vec`

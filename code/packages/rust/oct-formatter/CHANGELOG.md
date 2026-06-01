# Changelog — `oct-formatter`

## 0.1.0 — 2026-06-01 (OCT-FMT01 — initial Oct formatter)

Initial release.  Sibling of `nib-formatter` 0.1.0,
`basic-formatter` 0.1.0, and `twig-formatter` 0.2.0.  **Completes
the formatter phase** (task #42 part 3 of 3).

### What's here

- `FormatError` enum (reserved).
- `format(source) -> Result<String, FormatError>`.
- `format_tokens(tokens) -> String`.
- `Config { print_width, indent_width }`.

### Canonical form (V1)

- 2-space indent inside `{...}` blocks.
- Space around binary operators (`+ - * / = == != < <= > >= && || & | ^`).
- Space after `,`, `;`, `:`.
- No space inside `()`.
- Newline after `{` and `;` (when inside a block).
- Newline before `}`.
- Single trailing newline.
- Line comments (`// ...`) preserved verbatim — Oct's lexer
  surfaces them as `LINE_COMMENT` tokens (unlike BASIC's lexer
  which discards REM payloads).
- Idempotent.

### Tests

- 9 unit tests covering minimal main, single-statement body,
  binary-op spacing, comma/semicolon follower spacing, paren
  grouping, brace indentation, multi-statement bodies, line
  comment preservation, and idempotence.

# Changelog — `nib-formatter`

## 0.1.0 — 2026-06-01 (NIB-FMT01 — initial Nib formatter)

Initial release.  Token-stream → canonical Nib source.  Sibling of
`basic-formatter` 0.1.0 and `twig-formatter` 0.2.0.

### What's here

- `FormatError` enum (reserved for future failure modes).
- `format(source) -> Result<String, FormatError>` — primary entry.
- `format_tokens(tokens) -> String` — reuse a lex pass.
- `Config { print_width, indent_width }` — layout tuning.

### Canonical form (V1)

- 2-space indentation inside `{ … }` blocks.
- Space around binary operators (`+ - * / = == != < <= > >= && || & | ^`).
- Space after `,`, `;`, `:`.
- No space inside `()`.
- Newline after `{` (open block) and after `;` (statement end) when
  inside a block body.
- Newline before `}` (close block) so the closing brace lands on
  its own indented line.
- Single trailing newline (POSIX convention).
- Idempotent.

### Tests

- 8 unit tests covering minimal `fn main`, single-statement bodies,
  operator spacing, comma/semicolon follower spacing, paren
  grouping, brace indentation, multi-statement bodies, and
  idempotence.

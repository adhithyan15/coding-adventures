# Changelog — `basic-formatter`

## 0.1.0 — 2026-06-01 (BASIC-FMT01 — initial Dartmouth BASIC formatter)

Initial release.  Token-stream → [`format_doc::Doc`] → canonical
source.  Sibling of `twig-formatter` 0.2.0.  Part 1 of 3 of the
formatter phase (task #42).

### What's here

- `FormatError` enum (currently just `Lex(String)` for tokenisation
  failures).
- `format(source: &str) -> Result<String, FormatError>` — the
  primary entry point.
- `format_tokens(tokens: &[Token]) -> String` — reuse a lex pass.
- `Config { print_width, indent_width }` for layout tuning.

### Canonical form (V1)

| Rule | Example |
|------|---------|
| Keywords uppercased | `let a = 1` → `LET A = 1` |
| BASIC identifiers uppercased | `print a` → `PRINT A` |
| Single space between tokens | `LET  A  =  1` → `LET A = 1` |
| Space after `,` `;` | `PRINT A,B` → `PRINT A, B` |
| No space inside parens | `SQR ( X )` → `SQR(X)` |
| `REM` line preserved verbatim | `10 REM hello` → `10 REM hello` |
| Single trailing newline | always |

### How it works

The formatter walks the token stream line-by-line, building a
`format_doc::Doc` for each logical BASIC line with the appropriate
spacing rules.  Each line becomes a separate doc realised
independently — there's no horizontal-folding decision to make
because BASIC's grammar is one-statement-per-line.

### Tests

- 8 unit tests covering:
  - Trivial program (`10 END` → `10 END\n`)
  - Keyword uppercasing
  - Identifier uppercasing
  - Single-space normalisation
  - Comma/semicolon follower spacing
  - Paren grouping
  - REM preservation
  - Idempotence (formatting an already-formatted program is a no-op)

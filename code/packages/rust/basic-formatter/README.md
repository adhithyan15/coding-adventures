# `basic-formatter` — canonical Dartmouth BASIC pretty-printer

Token-stream → [format-doc](../format-doc) `Doc` → canonical
source.  The prettier/rustfmt equivalent for the BASIC authoring
experience.

The BASIC counterpart to `twig-formatter`.  First piece of the
formatter phase (task #42 part 1 of 3) and the wired-in formatter
for `basic-lsp-bridge`'s `format_fn`.

## Canonical form

| Rule | Example |
|------|---------|
| Keywords uppercased | `let a = 1` → `LET A = 1` |
| BASIC identifiers uppercased | `print a` → `PRINT A` |
| Single space between tokens | `LET  A  =  1` → `LET A = 1` |
| Single space around `,` `;` follower | `PRINT A,B` → `PRINT A, B` |
| No space inside parens | `SQR ( X )` → `SQR(X)` |
| `REM` keyword preserved (payload discarded) | `10 REM hello` → `10 REM` (the BASIC lexer drops the REM payload) |
| Single trailing newline | always |

## Why a formatter

Editors driving the BASIC LSP server can now offer
`textDocument/formatting` and `textDocument/rangeFormatting` (once
`basic-lsp-bridge` is updated to set `format_fn`).  Saves the
"Format Document" command from being a no-op for BASIC users.

## Public API

- `format(source: &str) -> Result<String, FormatError>` — the
  common case.  Lex + format.
- `format_tokens(tokens: &[Token]) -> String` — when callers
  already have a token stream.

## Position model

The formatter preserves logical line structure (one BASIC line per
output line) — it never breaks a statement across newlines, and
never joins two statements onto one line.

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.

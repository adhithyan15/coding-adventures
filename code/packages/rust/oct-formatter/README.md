# `oct-formatter` — canonical Oct pretty-printer

Token-stream → canonical Oct source.  The prettier/rustfmt
equivalent for Oct.

**Final piece** of the formatter phase (task #42 part 3 of 3).
Sibling of `nib-formatter`, `basic-formatter`, and
`twig-formatter`.

## Canonical form (V1)

| Rule | Example |
|------|---------|
| 2-space indent inside `{ }` blocks | rustfmt-style |
| Space around binary operators | `1+2` → `1 + 2` |
| Space after `,` `;` `:` | `f(a,b)` → `f(a, b)` |
| No space inside `( )` | `f(x)` → `f(x)` |
| Newline after `{` and `;` (inside a block) | one statement per line |
| Newline before `}` | closing brace on its own indented line |
| Single trailing newline (POSIX) | always |
| Idempotent | `format(format(x)) == format(x)` |
| Line comments dropped (lexer limitation) | `// hello` is consumed by the lexer's trivia pass and not surfaced — same behavior BASIC's REM has |

## Public API

- `format(source) -> Result<String, FormatError>` — common case.
- `format_tokens(tokens) -> String` — when callers have a token stream.
- `format_tokens_with_config(tokens, config) -> String` — custom config.

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.

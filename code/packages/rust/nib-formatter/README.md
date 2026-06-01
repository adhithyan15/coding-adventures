# `nib-formatter` — canonical Nib pretty-printer

Token-stream → canonical source.  The prettier/rustfmt equivalent
for Nib.

Sibling of `twig-formatter` and `basic-formatter`.  Part 2 of 3 of
the formatter phase (task #42).

## Canonical form

| Rule | Example |
|------|---------|
| 2-space indentation inside `{ }` | `fn f() { let x: u8 = 1; }` → multi-line indented |
| Space around binary operators | `1+2` → `1 + 2` |
| Space after `,` `;` `:` | `f(a,b)` → `f(a, b)` |
| No space inside `( )` | `f(x)` → `f(x)` |
| Newline after `{` and `;` (within block bodies) | one statement per line |
| Newline before `}` | closing brace on its own line |
| Single trailing newline (POSIX) | always |
| Idempotent | `format(format(x)) == format(x)` |

## Public API

- `format(source) -> Result<String, FormatError>` — common case.
- `format_tokens(tokens) -> String` — when callers have a token stream.
- `Config { print_width, indent_width }` — layout tuning.

## Versions

- `0.1.0` — initial release.

See [CHANGELOG.md](./CHANGELOG.md) for details.

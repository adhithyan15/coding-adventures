---
category: Rust
---

# The JSON grammar lexer preserves string escapes so AST consumers must decode them

`coding_adventures_json_parser::try_parse_json` retains numeric token text,
which is necessary for exact `u64` fixture values, but JSON string token values
still contain encoded escapes such as the two characters `\\` and `n` for a
newline. Comparing such a token directly with an API string that contains an
actual newline makes byte-exact fixture checks fail for the wrong reason. Decode
string token text with the repository JSON value/parser support while keeping
integer conversion on the original AST token text; do not route large integers
through a narrower or floating-point JSON value representation.

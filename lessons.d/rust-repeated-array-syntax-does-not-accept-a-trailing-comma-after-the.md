---
category: Rust
---

# Rust repeated-array syntax does not accept a trailing comma after the length

Two test fixtures split `[value; length]` over multiple lines and accidentally
put a comma after `length`. Rust's repeated-array form is one semicolon
expression, not a comma-separated list, so the parser rejects `[value;
length,]`. Keep the closing bracket immediately after the length expression;
let `rustfmt` choose the final line wrapping.

---
category: Rust
---

# A Rust lexer may skip a #! shebang only when the next significant token is not [, because #! [attr] is an inner attribute

The lexer skipped line 1 whenever a file started with `#!` and not `#![`. But
rustc strips a shebang only when the next significant token after `#!` is not
`[`: `#! [allow(dead_code)] unsafe fn evil() {}` (spaces) and
`#!/* c */[attr]` (a comment between) are inner attributes followed by code, so
the checker silently dropped a whole line of real source, `unsafe` included, and
printed PASS.

Fix: after `#!`, skip whitespace and (nested) comments; if the next character is
`[` it is an attribute and nothing is skipped, otherwise skip the line. Assert
the `unsafe` finding itself in the test (a bare rule-id check passed on the old
code because the root also lacked `forbid(unsafe_code)`).

Do differently: a "skip this region" rule in a security lexer must be copied
from the compiler's own condition, not from the common case, and every skipped
region needs a test that puts a violation immediately after it.

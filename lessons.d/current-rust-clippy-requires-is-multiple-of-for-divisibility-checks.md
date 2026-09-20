---
category: Rust
---

# Current Rust Clippy requires is multiple of for divisibility checks

The repository's current Rust/Clippy toolchain denies
`manual_is_multiple_of`, including familiar expressions such as `year % 4 ==
0`. Use the integer `is_multiple_of` method (and negate it for non-multiples)
before running package Clippy with `-D warnings`.

---
category: Rust
---

# Derived PartialEq is not automatically const-callable

A tag predicate was declared `const fn`, but it compared a derived enum with
`==`; that `PartialEq` implementation was not const and compilation failed.
Use an ordinary function for predicates that call non-const trait methods, or
match variants explicitly only when a const context is genuinely required.

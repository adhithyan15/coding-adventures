---
category: Rust
---

# Use a closure to map array lengths instead of a type-qualified len function

An inventory helper attempted to count nested Rust array storage with
`iter().map(<[u8; 32]>::len)`. The compiler resolved `len` through iterator
traits rather than the array value and rejected the call because an array is
not an `ExactSizeIterator`.

Use `iter().map(|array| array.len())` when mapping an inherent method whose
receiver needs dereference coercion. The closure makes the receiver value and
coercion explicit, compiles on the repository toolchain, and still derives the
count from the concrete storage instead of duplicating a topology constant.

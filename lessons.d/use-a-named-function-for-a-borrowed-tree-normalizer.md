---
category: Rust
---

# Use a named function for a borrowed tree normalizer

The conditional identity helper initially used a closure returning a node
borrowed from its input. Rust inferred unrelated input/output lifetimes and
rejected the candidate before tests ran. A small local function expresses the
single elided borrowing lifetime directly. Preserve the compilation-failure
receipt separately; it is not an executed semantic test result.

TODO: what went wrong, what the fix was, and what to do differently. Replace this line.

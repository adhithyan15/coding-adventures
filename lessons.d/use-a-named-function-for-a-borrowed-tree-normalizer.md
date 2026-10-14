---
category: Rust
---

# Use a named function for a borrowed tree normalizer

The conditional identity helper initially used a closure returning a node
borrowed from its input. Rust inferred unrelated input/output lifetimes and
rejected the candidate before tests ran. A small local function expresses the
single elided borrowing lifetime directly. Preserve the compilation-failure
receipt separately; it is not an executed semantic test result.

Parsed declaration fixtures also need their grammar-required separator before
the enclosing block's statement. Keep a minimal valid enclosing assignment,
so a guard test reaches the guard rather than failing in the parser.

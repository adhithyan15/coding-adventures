---
category: Rust
---

# Compiler-checked hostile fixtures must build the mutated complete crate

Parsing a mutated Rust source file proves only syntax. It does not prove that
the hostile form is a realistic bypass: a new struct field, import, or helper
can parse while failing name resolution or construction in the real crate.

For structural authority checks, copy the complete production source set into
a temporary crate, reproduce its production dependency graph, apply exactly
one hostile mutation, and require `cargo check --lib` to succeed before the
validator rejects it. Keep one shared temporary target directory so a matrix
of mutations remains practical without weakening the compile requirement.

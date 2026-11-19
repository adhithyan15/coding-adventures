---
category: Rust
---

# Pin Rust slice-chunk idioms to the repository's Clippy toolchain

A test fixture parser used `.chunks_exact(2)` after checking that the input hex
length was even. The repository's Rust 1.100 Clippy flags that form with
`chunks_exact_to_as_chunks` under `-D warnings`, even though the parser's
validation and behavior are correct. The same fixture test also carried elidable
explicit lifetimes and a unit-valued `let _ = ...` that the canonical Clippy gate
rejects.

Do differently: run the crate's canonical all-targets Clippy command with the
pinned toolchain, and use `slice.as_chunks::<2>().0.iter()` after the existing
even-length check. Keep the check before chunking so odd-length inputs retain
their explicit rejection. Let Rust elide straightforward lifetimes and call
unit-returning validators directly.

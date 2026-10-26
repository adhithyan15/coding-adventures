---
category: Rust
---

# Rust 1.99 prefers `as_chunks` for constant-width slices

A package passed strict Clippy locally, but CI's newer Rust 1.99 toolchain
enabled `chunks_exact_to_as_chunks` under `-D warnings`. A loop parsing fixed
five-byte machine words with `bytes.chunks_exact(5)` therefore failed only on
the Ubuntu lane after publication.

When input alignment has already been validated and the chunk width is a
compile-time constant, iterate over `bytes.as_chunks::<N>().0`. Besides staying
warning-clean, each loop item is an exact `[T; N]` reference, so the parser's
fixed transport shape is represented in the type rather than by repeated
slice indexing. Run strict checks with the repository's CI Rust version when a
new stable release has landed; a clean older local toolchain is not evidence
that warning-denied CI will remain clean.

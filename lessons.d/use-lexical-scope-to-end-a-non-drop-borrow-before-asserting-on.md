---
category: Rust
---

# Use lexical scope to end a non-Drop borrow before asserting on its owner

Calling `drop(value)` solely to end a mutable borrow triggers Clippy's
`drop_non_drop` lint when the wrapper has no `Drop` implementation, even though
the borrow checker accepts the lifetime boundary. Put the wrapper and all of
its assertions inside a lexical block instead, return only non-borrowing test
data such as a formatted redacted string, and assert on the original owner
after the block. This makes the lifetime boundary explicit without suggesting
that the wrapper has destruction behavior.

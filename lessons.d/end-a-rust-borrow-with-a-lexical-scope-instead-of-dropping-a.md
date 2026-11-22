---
category: Rust
---

# End a Rust borrow with a lexical scope instead of dropping a non-Drop evidence type

A test explicitly called `drop(evidence)` so it could inspect the caller-owned
buffer after the evidence's borrow ended. The code compiled and the borrow did
end, but repository Clippy with `-D warnings` rejected `drop_non_drop` because
the evidence type has no `Drop` implementation.

Put the evidence assertions in a smaller lexical block and return only the
public length needed by the outer assertion. The block makes the borrow end
without an explicit `drop` call and keeps the evidence type free of an
unnecessary destructor.

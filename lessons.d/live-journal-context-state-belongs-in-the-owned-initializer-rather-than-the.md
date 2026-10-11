---
category: Rust
---

# Live journal context state belongs in the owned initializer rather than the wire destructuring pattern

An ambiguous patch anchor inserted `active: Vec::new()` and `poisoned: false`
into `let Wire { ... }`, producing illegal patterns and missing owned fields.
Initialize these internal fields only in `Ok(Self { ... })`; the imported wire
contains closed evidence, never a caller-supplied active stack or poison flag.
Anchor the patch to the initializer's surrounding statement, not `events` alone.

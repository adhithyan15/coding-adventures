---
category: Rust
---

# Rewriting an index loop over a Vec into iter().take(N) silently drops the out-of-bounds panic; keep fail-closed with an explicit length check

The Rust 1.100 `needless_range_loop` sweep rewrote
`for address in 0..MEMORY_BYTES { oracle.memory[address] ... }` into
`oracle.memory.iter().enumerate().take(MEMORY_BYTES)` in
`ibm-system370-gatelevel`. The index loop panicked on a short `Vec`, which was
an accidental fail-closed check. `take` stops quietly at the shorter length, so
a truncated oracle compared fewer bytes and could be reported equal.

Fix: when the bound is not fixed by the type (a `Vec`, not an array), add an
explicit `if v.len() < N { return false; }` ahead of the loop, and pin it with
a test whose only difference is the shortened `Vec`. When a lint rewrite turns
an indexing loop into an iterator, ask what the old out-of-bounds panic was
protecting and keep that guarantee on purpose.

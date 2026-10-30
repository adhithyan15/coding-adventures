---
category: Rust
---

# Rust const-generic hex helpers need explicit output lengths at call sites

A helper returning `[u8; N]` cannot always infer `N` when the result is compared
through a slice, because the slice erases the array length before type inference
has a concrete target. Calls such as `decode_hex("...")` therefore fail with
E0284 even when the hex string visually implies the length. Spell the const
argument at the call site (`decode_hex::<32>(...)`) or bind the result to an
explicit `[u8; 32]` type. Prefer the explicit const argument in vector tests so
the required protocol length stays visible beside every fixture.

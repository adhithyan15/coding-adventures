---
category: Cryptography & security review
---

# Repository zeroize word arrays element by element because only byte arrays implement the trait

The fixed-memory SHA-256 hardening initially called `.zeroize()` directly on
`[u32; 8]` and `[u32; 64]`. The repository-owned zeroize crate implements the
trait for primitive integers and fixed byte arrays, but not for fixed arrays of
other primitive types, so the crate failed to compile before its tests ran.

For repository-owned cryptographic word state, add a small typed scrub helper
that iterates over the array and invokes `Zeroize` on each word. Read the actual
zeroize trait implementations before assuming an upstream crate's blanket
array implementation exists.

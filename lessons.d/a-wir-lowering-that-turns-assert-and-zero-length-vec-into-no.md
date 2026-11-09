---
category: Compiler / VM / language pipeline
---

# A WIR lowering that turns assert and zero-length vec into no-op or out-of-bounds store makes proofs of the real function fail or lie

While proving `ct_select_bytes` from the Warrant spec (VER04h), the Rust-to-WIR
lowering turned `assert_eq!(a.len(), b.len())` into a no-op, so the length
mismatch path became an out-of-bounds scan, and `vec![0u8; n]` emitted a
first-element store that wrote past a zero-length buffer. Neither showed up in
the earlier differential tests because they only used non-empty, equal-length
buffers.

**Do instead:** when a proof or differential test covers a function, include the
boundary shapes (length 0, unequal lengths, the panic path) and compare traps
with the real function's panics. Lower active assertions to a branch into a
trap; never to a no-op. Also note that the symbolic executor reports a trap as
an error rather than a path outcome, so trapping shapes are counted, not
proved, and must be covered by another suite.

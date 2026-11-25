---
category: Rust
---

# A masked branchless select lets LLVM prove the mask is 0 or 0xFF and compile it to a pointer select plus memcpy

`ct_select_bytes` computed `out[i] = b[i] ^ ((a[i] ^ b[i]) & mask)` with
`mask = 0u8.wrapping_sub(choice as u8)`. That is branchless *source*, but at
`-C opt-level=3` LLVM knows `mask` is exactly 0x00 or 0xFF. So it recognises the
loop as "copy whichever slice `choice` picks" and emits
`select i1 %choice, ptr %a, ptr %b` feeding `memcpy`. The compiled function
reads only the chosen buffer, and the *addresses* it touches depend on the
secret: a cache side channel. The crate's IR gate only checked the equality
scan, and the shipped assurance list still claimed
`cc.llvm-ir.addresses-public`. Found by the LV-007b pre-release security review
(SEC-013).

**Fix:** hide the mask's value from the optimizer by passing it through the
`core::hint::black_box`, and pin it with an optimized-IR test (`black_box` is
only a hint, so the test is what makes it binding). A volatile read of a local
was tried first and broke the WIR proofs ("Symbolic integer used as pointer"):
the proof pipeline cannot take the address of a scalar local. The test fails on any pointer `select`, any
`memcpy`/`memmove`, or any branch on `choice` in the function body.

**Next time:** for every constant-time primitive, check the *optimized* IR or
assembly, not the source. A value the compiler can prove is 0/all-ones turns
mask arithmetic back into selects, branches or bulk copies.

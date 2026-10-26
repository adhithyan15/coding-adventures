---
category: Compiler / VM / language pipeline
---

# Architectural shift counts require explicit width and sign-bit semantics

Rust's `wrapping_shl` and `wrapping_shr` reduce a count modulo the host integer
width. System/360 uses all six low address bits, so a 32-bit shift by 32 through
63 must clear a logical operand or sign-fill an arithmetic-right operand rather
than repeat counts zero through 31. Arithmetic-left shifts additionally retain
the original sign bit while testing every displaced numeric bit for overflow.

Implement the architecture's full count domain explicitly with checked shifts
and width-specific fallback values. For sign-preserving left shifts, shift only
the numeric field, restore the original sign bit, and perform overflow checking
in a wider integer. Test the boundary counts 0, width - 1, width, and 63.

---
category: Rust
---

# Signed parallel lane flags require the wide sum sign

The first gate implementation of signed byte-lane addition derived GE from the
sign bit of the truncated eight-bit result. Arm defines GE from whether the
mathematical signed sum is nonnegative, so an overflowing positive sum can
have a negative truncated sign while GE must remain set. Sign-extend both
operands, add at one extra bit of width, and derive the predicate from that
wide sum. Include deterministic cross-product differential vectors, not only
hand-picked non-overflow examples.

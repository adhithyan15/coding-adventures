---
category: Rust
---

# Binary floating equality must include sign except for zero

A gate comparison initially treated equal exponent-and-fraction fields as an
equal floating value. That incorrectly collapsed `-1.0` and `+1.0`, even
though the magnitude wiring was correct. Floating equality needs a same-sign
wire in addition to equal exponent and fraction; only the explicitly detected
signed-zero pair may bypass that sign comparison. Include opposite-sign,
equal-magnitude values in every floating compare differential matrix.

---
category: Testing & coverage
---

# A matrix row whose operation is its own inverse cannot detect an encoding bug

The native x86-64 and AArch64 backends lowered `neg_f64` (IIR `neg` on a
real) as an integer two's-complement negate of the f64's raw bits. That
miscompiles every real negation: `LET X = 2.5: PRINT -X` printed `-1.75`,
because 2.5 is 0x4004… and its integer negation 0xBFFC… is −1.75. Dartmouth
BASIC emits this op for every unary minus on a real.

The lang-aot matrix had a row "proving" `neg`(f64) on NativeAot:
`PRINT ABS(-42)`. It negates twice (once for the literal `-42`, once inside
`ABS`), and a two's-complement negate is its own inverse, so the wrong
encoding round-tripped to the right `42`. The row passed for the wrong reason.

**Fix:** flip only bit 63 (`xor` / `EOR` with `1<<63`), and add an asymmetric
row: `LET X = 2.5` / `PRINT -X` → `-2.5`.

**Do differently:** when a test is meant to prove an operation, make sure its
input cannot pass through a wrong encoding unchanged.
- Negation, bitwise NOT and byte swap are their own inverse, so test each
  applied *once*, to a value whose wrong encoding prints differently. 2.5 →
  −1.75 shows the bug at once; −42 → 42 via a double negate never does.
- Pick operands asymmetric in bits as well as in value: `+0.0` should become
  `−0.0`, and a narrow integer should not be its own wrap-around image.

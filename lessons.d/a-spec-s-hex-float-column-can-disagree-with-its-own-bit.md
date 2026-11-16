---
category: Testing & coverage
---

# A spec's hex-float column can disagree with its own bit-pattern column, so build every constant from its bits and let an independent host catch the typo

PHY00a (trig convergence) lists each reduction constant three ways: bit pattern,
hex float, decimal. The hex float of `PIO2_3` read `0x1.84698p-48`; the bit pattern
`0x3CF8469880000000` is `0x1.846988p-48` (one hex digit lost). The bits were right,
but the first C cross-check transcribed the hex float and disagreed with the Python
soft-float model and the Rust check on `sin(0x419fffffffffffff)` by 840 ulp. The
disagreement was found only because the vector generator compares three unrelated
implementations of the same algorithm and stops on any difference.

What to do differently: when a spec gives a constant in more than one spelling, say
which one is authoritative (here, the bits) and build every constant in code from it
(`F(0x...ull)`, `f64::from_bits`, `struct.unpack`), never from a second spelling.
Have a test decode each constant's own bits and compare the other spellings to it.
Treat "two independent transcriptions disagree" as a defect in the *text* until
proven otherwise, and fix the text.

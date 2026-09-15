## 0.215.0 — 2026-08-27 — nested checked integer exponents

Variable-free checked integer exponent evaluation now recurses through bounded
power operands. Integer lowering and selector identity analysis therefore keep
expressions such as `x ^ ((2 ^ (2 - 1)) div 2)` on the exact integer path.
Overflow, division by zero, negative, oversized, dynamic, and non-unit forms
continue to fail closed.


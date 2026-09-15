## 0.214.0 — 2026-08-27 — checked integer exponent expressions

Bounded exponent lowering now accepts variable-free checked integer arithmetic
operands, retaining the integer unrolled-power path instead of widening to a
runtime real power. Integer selector identity analysis uses the same rule when
the complete exponent chain is one. Overflow, division by zero, dynamic,
negative, oversized, and non-unit forms remain conservative.


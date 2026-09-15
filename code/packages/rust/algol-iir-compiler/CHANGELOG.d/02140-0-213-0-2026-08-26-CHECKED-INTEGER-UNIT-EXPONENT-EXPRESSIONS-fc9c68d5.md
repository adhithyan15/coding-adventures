## 0.213.0 — 2026-08-26 — checked integer unit exponent expressions

Real selector identity recognition now also accepts variable-free integer
literal arithmetic exponent operands when checked `i64` evaluation leaves
each operand nonnegative and the complete bounded chain equal to one.
Overflow, division by zero, dynamic, non-integral, and non-unit forms remain
conservative.


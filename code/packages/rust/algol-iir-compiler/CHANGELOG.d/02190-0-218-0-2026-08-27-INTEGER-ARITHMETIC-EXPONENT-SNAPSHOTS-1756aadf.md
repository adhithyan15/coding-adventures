## 0.218.0 — 2026-08-27 — integer arithmetic-exponent snapshots

Straight-line integer snapshot evaluation now retains bounded powers whose
exponents are variable-free checked integer arithmetic, including nested
powers. This matches the existing integer power lowerer while overflowing,
negative, oversized, and dynamic exponent forms remain conservative.


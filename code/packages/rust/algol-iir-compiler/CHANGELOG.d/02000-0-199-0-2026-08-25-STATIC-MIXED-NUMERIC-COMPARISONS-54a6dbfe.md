## 0.199.0 — 2026-08-25 — static mixed numeric comparisons

Bounded static condition evaluation now applies one comparison-wide numeric
coercion decision, allowing finite real operands to widen tracked integer
peers that are exactly representable in binary64. Inexact widening and
non-finite values remain conservative.


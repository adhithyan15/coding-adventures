## 0.202.0 — 2026-08-25 — real additive-negative-zero selector writes

Bounded static while analysis now recognizes addition of a negative real-zero
literal as an exact finite-real selector identity, including left-associative
chains and grouped literals. Positive-zero addition and negative-real-zero
subtraction remain conservative because either can change the sign bit of
`-0.0`; integer `-0` continues to mean positive numeric zero.


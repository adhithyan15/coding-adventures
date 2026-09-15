## 0.201.0 — 2026-08-25 — real subtractive-zero selector writes

Bounded static while analysis now recognizes subtraction of positive numeric
zero as an exact finite-real selector identity, including left-associative
chains and grouped or explicitly positive zero literals. Addition of positive
zero and subtraction of negative real zero remain conservative because either
can change the sign bit of `-0.0`.


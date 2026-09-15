## 0.205.0 — 2026-08-26 — nested real zero identities

Exact zero recognition now evaluates bounded literal-only addition and
subtraction subexpressions with IEEE-754 zero-sign semantics. Grouped positive
zero results may be subtracted and grouped negative-zero results may be added
without invalidating finite-real selector dependencies; opposite signs remain
conservative.


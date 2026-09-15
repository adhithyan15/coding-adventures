## 0.188.0 — 2026-08-25 — even boolean negation selector writes

Bounded static while analysis now recognizes even chains of unary `not` as
exact boolean identity writes. Odd chains remain changing, binary identities
under an even negation chain retain their existing structural proof, and the
ten-level selector-dependency cap is unchanged.


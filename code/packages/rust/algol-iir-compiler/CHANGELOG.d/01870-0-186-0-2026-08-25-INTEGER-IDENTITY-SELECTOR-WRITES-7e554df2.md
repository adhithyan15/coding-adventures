## 0.186.0 — 2026-08-25 — integer identity selector writes

Bounded static while analysis now recognizes exact structural integer identity
writes using `+ 0`, `- 0`, `* 1`, and `div 1`, including valid commutative
forms. These assignments preserve transitive selector dependencies without
widening the ten-level recursion cap; non-identities, real arithmetic, and
unsupported computed writes still fail closed.


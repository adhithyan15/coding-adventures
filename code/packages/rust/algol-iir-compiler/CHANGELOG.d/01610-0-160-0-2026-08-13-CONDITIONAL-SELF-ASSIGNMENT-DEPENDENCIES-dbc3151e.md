## 0.160.0 — 2026-08-13 — conditional self-assignment dependencies

Capped `while` body-effect analysis now recognizes a conditional assignment as
idempotent when both leaves are the same bare scalar. Differing or computed
leaves remain conservative.


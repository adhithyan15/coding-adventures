---
category: Testing & coverage
---

# Keep ALGOL conditional branch types aligned in proof fixtures

A legacy-proof retention fixture used a real `(0.5+0.5)` offset in one arm and
integer offsets in the other. ALGOL correctly rejected the conditional's
real/integer branch mismatch before testing the intended proof boundary. Use
real literals in both arms for the legacy-real case, preserve the failed
receipt, and keep the ordinary integer two-unit case separate. Do not relax
production conditional typing to make the fixture pass.

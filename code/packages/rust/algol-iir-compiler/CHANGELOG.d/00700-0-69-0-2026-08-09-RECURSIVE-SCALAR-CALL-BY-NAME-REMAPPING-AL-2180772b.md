## 0.69.0 — 2026-08-09 — recursive scalar call-by-name remapping (AL7)

Direct and mutually recursive call-by-name procedures can now remap each
scalar formal to any active same-typed scalar formal, such as a recursive
`flip(n, y, x)`. The compiler tracks in-flight specialised siblings by their
canonical captured binding map, so a finite remapping cycle reuses the right
sibling while preserving each original caller expression and assignable write
target. Changed recursive scalar expressions still need a dynamic thunk frame
and remain rejected.


## 0.68.0 — 2026-08-09 — recursive scalar call-by-name forwarding (AL7)

A direct specialised sibling can now re-enter itself or another in-flight
sibling when every scalar name formal is forwarded unchanged under its original
formal name. This preserves the initial caller expression and assignable write
target, so direct and mutual recursion retain scalar read/write semantics
without erasing name passing to a value. Changed recursive scalar actuals still
need a dynamic thunk frame and remain rejected.


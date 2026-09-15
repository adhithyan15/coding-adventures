## 0.41.0 — 2026-07-31 — nested scalar-formal capture

A nested procedure may now read and assign an enclosing scalar `value`
parameter. The outer procedure copies the incoming typed value into a
compiler-owned global before entering the nested sibling function, which then
reloads and updates that same value. Shadowing formals remain local rather than
capturing an enclosing block scalar.


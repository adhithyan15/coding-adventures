## 0.39.0 — 2026-07-31 — nested array-formal capture

A procedure nested inside another procedure may now read and write an outer
`integer`, `real`, or `string` array `value` parameter. The outer frame copies
the incoming typed handle, every lower bound, and each non-final row-major
stride into compiler-owned globals; the nested sibling IIR function reloads the
same descriptor before each access. This preserves multidimensional declared
index spaces and aliases the caller's storage.


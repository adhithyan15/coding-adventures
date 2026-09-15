## 0.35.0 — 2026-07-31 — AL8 array value parameters

One-dimensional `integer`, `real`, and `string` array `value` parameters now
cross a procedure call as a typed IIR descriptor pair: the backing-storage
handle and the actual array's declared lower bound. The callee rebinds that
descriptor in its fresh scope, so ordinary subscript lowering preserves the
actual array's index space and writes remain visible to the caller. Captured
and `own` array actuals reload both descriptor fields from typed globals.

Array actuals must be bare, element-type-compatible, one-dimensional variables;
multidimensional and by-name array parameters remain unsupported. The frontend
regressions cover integer, real, string, and captured integer descriptors plus
the multi-dimensional rejection case.


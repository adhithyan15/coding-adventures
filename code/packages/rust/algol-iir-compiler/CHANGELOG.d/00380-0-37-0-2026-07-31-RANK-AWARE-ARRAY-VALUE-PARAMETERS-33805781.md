## 0.37.0 — 2026-07-31 — rank-aware array value parameters

`integer`, `real`, and `string` array `value` parameters now accept
multidimensional actuals. The compiler infers a formal's rank from its indexed
uses in the procedure body, then lowers the call ABI to the typed array handle
plus every lower bound and each non-final row-major stride. This keeps the
callee's `a[i,j,...]` accesses in the caller's declared index space, including
nonzero and negative lower bounds; writes still alias the actual storage.

One-dimensional descriptors retain their existing handle-plus-lower-bound ABI.
The compiler rejects rank-mismatched actuals and formals used with inconsistent
subscript counts. Regressions cover 2-D descriptor layout, 3-D VM execution,
and the diagnostics.


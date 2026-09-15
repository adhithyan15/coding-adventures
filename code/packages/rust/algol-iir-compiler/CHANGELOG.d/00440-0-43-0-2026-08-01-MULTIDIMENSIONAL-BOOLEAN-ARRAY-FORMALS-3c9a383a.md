## 0.43.0 — 2026-08-01 — multidimensional boolean array formals

Rank-aware boolean array value formals now have direct regression coverage:
the descriptor preserves two non-unit lower bounds and the outer row-major
stride, so writes in the procedure select the same cells as the caller.


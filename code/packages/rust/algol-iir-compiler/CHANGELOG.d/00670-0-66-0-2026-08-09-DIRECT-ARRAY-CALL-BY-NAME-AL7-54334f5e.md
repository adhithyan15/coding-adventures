## 0.66.0 — 2026-08-09 — direct array call-by-name (AL7)

Direct array name formals now retain the existing typed array descriptor ABI:
bare array actuals pass their handle, every lower bound, and every row-major
stride to a specialised direct sibling. The descriptor aliases caller storage,
so element writes remain visible and a name array can be forwarded through a
nested direct call. Coverage includes nested two-dimensional forwarding with
non-unit lower bounds.


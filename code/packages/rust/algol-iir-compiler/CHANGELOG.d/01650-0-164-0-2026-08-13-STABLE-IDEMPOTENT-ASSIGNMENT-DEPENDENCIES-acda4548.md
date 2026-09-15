## 0.164.0 — 2026-08-13 — stable idempotent assignment dependencies

Checked static idempotent assignments may now reference known ordinary local
numeric or boolean scalars that are never assignment or loop-control targets
in the body. Written, controlled, global, array, by-name, string, unknown, and
non-matching dependencies remain conservative.


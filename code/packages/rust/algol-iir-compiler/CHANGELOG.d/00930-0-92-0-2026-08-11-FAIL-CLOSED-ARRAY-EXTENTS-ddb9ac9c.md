## 0.92.0 — 2026-08-11 — fail-closed array extents

Array declarations now require every run-time dimension extent to remain
positive before computing strides or allocating storage. Reversed bounds and
`upper - lower + 1` overflow take a portable bounds-trap path. Row-major stride
and total-length products are divided back and checked, preventing signed `i64`
overflow from becoming an undersized allocation.


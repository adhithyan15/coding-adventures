## 0.122.0 — 2026-08-11 — loop string-initialization joins

`for` statements now restore the definite scalar-string initialization state
from loop entry at exit. This rejects body-only initialization because a
`while` or `step` element may execute zero times, while preserving strings
initialized before the loop.


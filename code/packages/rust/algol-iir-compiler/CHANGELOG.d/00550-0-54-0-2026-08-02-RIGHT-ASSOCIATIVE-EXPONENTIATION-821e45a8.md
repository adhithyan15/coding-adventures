## 0.54.0 — 2026-08-02 — right-associative exponentiation

ALGOL exponentiation chains now lower from the right, so `2 ^ 3 ^ 2` evaluates
as `2 ^ (3 ^ 2)` = 512 rather than `(2 ^ 3) ^ 2` = 64. Literal-only chains still
use the bounded typed multiplication fast path.


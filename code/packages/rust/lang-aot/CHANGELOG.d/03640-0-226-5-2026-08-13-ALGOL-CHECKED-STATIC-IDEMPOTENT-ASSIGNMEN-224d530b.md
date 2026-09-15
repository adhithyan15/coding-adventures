## 0.226.5 - 2026-08-13 (ALGOL checked static idempotent assignments)

The seven-backend ALGOL matrix now proves that a checked self-only expression
such as `n := n + 0` preserves a capped while dependency when it exactly equals
the tracked value.


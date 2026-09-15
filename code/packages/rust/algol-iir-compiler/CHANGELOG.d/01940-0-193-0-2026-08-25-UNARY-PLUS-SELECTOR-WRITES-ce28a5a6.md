## 0.193.0 — 2026-08-25 — unary-plus selector writes

Bounded static while analysis now recognizes leading unary plus as the exact
no-op already implemented by expression lowering. Integer identity tails remain
stable, while unary minus continues to fail closed for both integer and real
selectors.


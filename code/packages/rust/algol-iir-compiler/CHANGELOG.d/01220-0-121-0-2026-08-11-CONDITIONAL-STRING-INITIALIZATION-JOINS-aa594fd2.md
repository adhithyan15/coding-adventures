## 0.121.0 — 2026-08-11 — conditional string initialization joins

Definite initialization for scalar strings now intersects the initialized-slot
sets from both statement-conditional exits. A then-only assignment no longer
makes a later read unsoundly valid, while assignments on both branches and an
already initialized path without `else` remain supported.


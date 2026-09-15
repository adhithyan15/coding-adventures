## 0.62.0 — 2026-08-02 — dynamic `own string` reassignment

Regression coverage now proves that an `own string` can retain its typed global
identity while repeated calls replace its runtime handle and forward the newest
value through a string formal.


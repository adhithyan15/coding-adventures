## 0.179.0 — 2026-08-14 — five-level selector dependencies

Bounded static while analysis may now follow five nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.


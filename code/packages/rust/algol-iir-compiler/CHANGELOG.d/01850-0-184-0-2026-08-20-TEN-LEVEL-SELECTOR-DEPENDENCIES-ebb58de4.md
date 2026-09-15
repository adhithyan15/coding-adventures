## 0.184.0 — 2026-08-20 — ten-level selector dependencies

Bounded static while analysis may now follow ten nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.


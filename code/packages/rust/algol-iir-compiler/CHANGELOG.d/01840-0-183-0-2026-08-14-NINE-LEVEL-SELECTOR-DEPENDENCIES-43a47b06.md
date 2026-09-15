## 0.183.0 — 2026-08-14 — nine-level selector dependencies

Bounded static while analysis may now follow nine nested conditional-selector
dependencies while retaining the fixed depth that makes cycles and longer
chains fail closed without general recursive effect inference.


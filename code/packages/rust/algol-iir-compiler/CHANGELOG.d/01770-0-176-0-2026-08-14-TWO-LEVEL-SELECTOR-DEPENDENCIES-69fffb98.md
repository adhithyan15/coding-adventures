## 0.176.0 — 2026-08-14 — two-level selector dependencies

Bounded static while analysis may now follow two nested conditional-selector
dependencies when each known predicate is stable and selects the preserving
leaf. The fixed depth still makes cycles and longer chains fail closed without
general recursive effect inference.


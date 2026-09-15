## 0.134.0 — 2026-08-13 — path-independent conditional predicates

An initial `while` conditional with an unknown selector now has a static result
when both branches independently prove the same boolean. Differing or unknown
branches remain conservative.


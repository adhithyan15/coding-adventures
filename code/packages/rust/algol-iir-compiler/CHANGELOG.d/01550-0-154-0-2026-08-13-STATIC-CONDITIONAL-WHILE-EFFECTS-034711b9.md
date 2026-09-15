## 0.154.0 — 2026-08-13 — static conditional while effects

Capped `while` dependency analysis now scans only the selected branch of a
variable-free statically decidable body conditional. Dynamic and
variable-dependent conditions continue to scan both branches conservatively.


## 0.151.0 — 2026-08-13 — standard-function while controls

Capped `while` control analysis now recognizes dependency names beneath exact
static standard-function calls without misclassifying the callee as a scalar.
User-declared procedures with the same spelling continue to shadow the built-in
and remain conservative.


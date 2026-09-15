## 0.161.0 — 2026-08-13 — static conditional assignment effects

Capped `while` body-effect analysis now examines only the selected leaf of a
variable-free statically known conditional assignment. A selected exact scalar
self-assignment preserves the dependency; a selected changing leaf and dynamic
selectors with differing leaves remain conservative.


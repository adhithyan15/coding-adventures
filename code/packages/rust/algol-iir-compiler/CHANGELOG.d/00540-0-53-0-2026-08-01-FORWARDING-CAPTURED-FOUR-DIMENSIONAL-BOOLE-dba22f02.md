## 0.53.0 — 2026-08-01 — forwarding captured four-dimensional boolean arrays

Regression coverage now proves that a nested procedure can forward a captured
four-dimensional `boolean array` value formal to a sibling array formal. The forwarding
call reloads the `array<bool>` handle, four non-unit lower bounds, and all three row-major
strides before the callee writes caller-visible checkerboard cells.


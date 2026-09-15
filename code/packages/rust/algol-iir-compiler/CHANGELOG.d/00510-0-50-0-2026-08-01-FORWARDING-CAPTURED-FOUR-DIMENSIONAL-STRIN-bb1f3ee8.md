## 0.50.0 — 2026-08-01 — forwarding captured four-dimensional string arrays

Regression coverage now proves that a nested procedure can forward a captured
four-dimensional `string array` value formal to a sibling array formal. The forwarding
call reloads the `array<str>` handle, four non-unit lower bounds, and all three row-major
strides before the callee writes the caller-visible dynamic string cells.


## 0.82.0 — 2026-08-11 — subscripted for controlled variables

Integer array elements may now serve as ALGOL for-clause controlled variables.
Their designators are re-evaluated for each assignment and read through the
existing bounds-checked typed `array_set`/`array_get` lowering.


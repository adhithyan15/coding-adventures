## 0.91.0 — 2026-08-11 — multidimensional coordinate bounds

Each array subscript coordinate is now bounds-checked before row-major
flattening. Dimension sizes are recovered from the existing flat array length
and adjacent descriptor strides, so invalid coordinates cannot alias a valid
neighboring row and the procedure ABI does not grow. Invalid reads and writes
reuse the shared backend `array_get`/`array_set` trap path.


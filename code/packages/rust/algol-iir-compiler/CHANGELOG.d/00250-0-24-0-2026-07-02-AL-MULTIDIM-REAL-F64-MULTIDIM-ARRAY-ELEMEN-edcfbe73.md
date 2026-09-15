## 0.24.0 — 2026-07-02 — AL-multidim-real: f64 multidim array elements (LANG-FULL)

Proves that the N-dimensional array machinery added in 0.23.0 carries **`real`
(f64) elements**, not just integers.  When AL-multidim first landed, f64
multidim elements were flagged as a follow-up; this closes that gap.

No compiler code changed — `emit_array_decl` already threads the declared
`elem_ty` (here `ScalarType::Real`) into `declare_array`, and `array_get`/
`array_set` ride the same 8-byte slots for f64 as for i64.  Only the flat-index
computation is multidim; the element storage is identical to a 1-D `real array`.

Two new unit tests (108 total):
- `two_d_real_array_roundtrips` — store four doubles into `M[1:2, 1:2]`, read
  `M[2,2]` back = 4.5
- `two_d_real_array_sum` — sum all four cells (1.5+2.5+3.5+4.5) = 12.0 via the
  f64 `add` path over multidim reads

The 7-backend proof lives in `lang-aot` `lang_matrix.rs` (fractional cells
summing to 42.0, floored via `entier`).


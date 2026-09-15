## 0.26.0 — 2026-07-02 — AL-multidim-bounds: arbitrary/negative lower bounds (LANG-FULL)

Proves that ALGOL's **arbitrary per-dimension lower bounds** (`[lo:hi]` with
`lo ≠ 1`, including `0` and negative) compose correctly with the multidim
row-major strides.  Each subscript is translated to `sub − lower` *before* the
stride is applied: `flat = Σ_d (sub[d] − lower[d]) * stride[d]`.  The 1:N cells
shipped so far never exercised `lower ≠ 1`, so this closes a real correctness
gap in the multidim index math.

No compiler code changed — `ArrayDim` already records a per-dimension
`lower_slot`, and `resolve_array_index` already emits the `sub − lower`
subtraction; this is a coverage-gap closure.

Two new unit tests (113 total):
- `two_d_array_arbitrary_lower_bounds` — `M[0:1, 2:4]`, flat index `(i)*3 + (j−2)`
- `two_d_array_negative_lower_bounds` — `M[−2:−1, 0:1]`, flat `(i+2)*2 + j`

The 7-backend proof lives in `lang-aot` `lang_matrix.rs` (`M[−1:1, 2:3]`, a
negative and a non-zero lower bound, summing to 42).


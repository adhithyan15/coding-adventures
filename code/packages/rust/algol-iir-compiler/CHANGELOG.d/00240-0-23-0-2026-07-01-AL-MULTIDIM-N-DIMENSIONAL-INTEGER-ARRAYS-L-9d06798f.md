## 0.23.0 — 2026-07-01 — AL-multidim: N-dimensional integer arrays (LANG-FULL)

**`ArrayDim` struct** (new): per-dimension record holding `lower_slot: String`
(the IIR variable that holds the declared lower bound) and `stride_slot:
Option<String>` (the IIR variable that holds `product(size[d+1..N-1])`; `None`
for the last dimension where the stride is 1 and the multiply is elided).

**`ArrayInfo` struct** (changed): `lower_slot: String` → `dims: Vec<ArrayDim>`
so multi-dimensional arrays carry one `ArrayDim` per subscript position.

**`emit_array_decl`** (rewritten): For each dimension, evaluates the declared
lower and upper expressions, computes `size_d = upper - lower + 1` via
`sub_i64` + `add_i64`.  Strides are computed right-to-left: the last dimension
has `stride_slot = None` (multiply elided); each outer dimension `d` emits
`stride_d = size_{d+1} * running_stride` via `mul_i64`.  Total allocation
length = `size_0 * stride_0` for N ≥ 2 (single `mul_i64`), or `size_0` for N
= 1 (unchanged 1D path).

**`resolve_array_index`** (rewritten): Validates that `subs.len() ==
info.dims.len()` (compile-time arity check; panics on mismatch).  For each
dimension `d`: computes `diff_d = sub_d - lower_d` via `sub_i64`; contributes
`diff_d * stride_d` via `mul_i64` when `stride_slot` is Some, else `diff_d`
directly.  Accumulates contributions via `add_i64` into a single flat 0-based
index.  No backend change required — the emitted IIR uses only
`alloc_array`/`array_set`/`array_get` with flat indices, identical to 1D.

**5 new unit tests** (106 total):
- `two_d_array_store_and_load` — 2×2 matrix, single cell round-trip
- `two_d_array_all_four_cells` — all four cells produce the correct flat indices
- `two_d_array_non_square` — 3×2 (6-element flat array)
- `two_d_array_filled_with_loops` — nested ALGOL `for` loops fill + sum a 2D array
- `rejects_wrong_subscript_count_for_2d_array` — compile-time arity check panics


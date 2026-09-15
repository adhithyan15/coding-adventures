## 0.25.0 — 2026-07-02 — AL-multidim-3D: 3-D integer arrays (LANG-FULL)

Proves the multidim array machinery is genuinely **N-dimensional**, not
hardcoded to 2-D.  For `integer array M[1:2, 1:2, 1:2]` the strides are computed
right-to-left at declaration: `stride[2] = 1` (elided), `stride[1] = size[2] =
2`, `stride[0] = size[1] * stride[1] = 4` — so subscript `M[i,j,k]` lowers to
the flat 0-based index `(i−1)*4 + (j−1)*2 + (k−1)` over a single `alloc_array`
of length 8.

No compiler code changed — `emit_array_decl`'s right-to-left stride loop and
`resolve_array_index`'s accumulation already handle any dimensionality; the
3-D case just walks the loop one more iteration.  This is a coverage-gap
closure.

Three new unit tests (111 total):
- `three_d_array_store_and_load` — `M[2,2,2]` (flat index 7, last of 8 cells)
- `three_d_array_all_eight_cells` — triple-nested loop fills all 8, reads three
  corners (flat 0/4/7)
- `three_d_array_non_cubic` — `M[1:2, 1:3, 1:4]` (24 elements) proves the general
  stride product (stride[0]=12, stride[1]=4)

The 7-backend proof lives in `lang-aot` `lang_matrix.rs`.


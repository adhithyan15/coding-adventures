## 0.168.0 — 2026-07-02 — BA-DIM-2D: Dartmouth BASIC 2D DIM array matrix proof cell (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): Dartmouth BASIC two-dimensional
array running on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

```basic
10 DIM A(1,2)
20 LET A(0,0) = 40
30 LET A(1,2) = 2
40 PRINT A(0,0) + A(1,2)
50 END
```

`DIM A(1,2)` is a 2×3 matrix (0-based inclusive) lowered to a single flat
`alloc_array` of 6 `f64` elements; `A(i,j)` folds through the row-major strides
`[3,1]` to the flat index `i*3 + j`. Stores `A(0,0)=40` (flat 0) and `A(1,2)=2`
(flat 5, the last cell — proving the row stride), then prints their sum ⇒ `42`.
No new IIR op, no backend change — same E5 array substrate as the BA3 1-D cell.
Requires `dartmouth-basic-iir-compiler` 0.35.0 / `-parser` 0.3.0.


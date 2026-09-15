## 0.169.0 — 2026-07-02 — AL-multidim-bounds: ALGOL 60 2D array with arbitrary/negative lower bounds (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60 2D integer array with a
**negative** lower bound on one axis and a non-zero one on the other, running on
all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

```algol60
begin integer array M[0-1:1, 2:3]; integer result;
  M[0-1, 2] := 40; M[1, 3] := 2;
  result := M[0-1, 2] + M[1, 3] end
```

`M[-1:1, 2:3]` has sizes `(3, 2)`, strides `[2, 1]`, so `M[i,j]` folds to
`(i−(−1))*2 + (j−2)`.  `M[-1,2]` is flat 0, `M[1,3]` is flat 5 (the last of 6
cells — proving both the per-dimension `sub−lower` subtraction and the row
stride).  The negative bound is written `0-1` since ALGOL number literals are
unsigned.  No new IIR op, no backend change.  Requires `algol-iir-compiler`
0.26.0.  Exit 42.


## 0.167.0 — 2026-07-02 — AL-multidim-3D: ALGOL 60 3D integer array matrix proof cell (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60 3D integer array running
on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit) — proves the
multidim lowering is genuinely N-dimensional, not hardcoded to 2-D.

```algol60
begin integer array M[1:2, 1:2, 1:2]; integer result;
  M[1, 1, 1] := 6; M[2, 1, 1] := 16; M[1, 2, 1] := 20;
  result := M[1, 1, 1] + M[2, 1, 1] + M[1, 2, 1] end
```

For `M[1:2, 1:2, 1:2]` the strides are `stride[0]=4, stride[1]=2, stride[2]=1`,
so `M[i,j,k]` → flat `(i−1)*4 + (j−1)*2 + (k−1)`. Three corner cells with
distinct flat indices (0, 4, 2) are stored to exercise every stride, then summed
= 42. Still only `alloc_array`/`array_set`/`array_get` with flat indices — no
backend change. Requires `algol-iir-compiler` 0.25.0. Exit 42.


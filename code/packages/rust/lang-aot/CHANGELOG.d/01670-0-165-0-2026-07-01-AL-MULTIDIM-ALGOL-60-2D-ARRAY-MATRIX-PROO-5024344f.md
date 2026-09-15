## 0.165.0 — 2026-07-01 — AL-multidim: ALGOL 60 2D array matrix proof cell (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60 2D integer array
running on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

```algol60
begin integer array M[1:2, 1:2]; integer result;
  M[1, 1] := 10; M[1, 2] := 20; M[2, 1] := 5; M[2, 2] := 7;
  result := M[1, 1] + M[1, 2] + M[2, 1] + M[2, 2] end
```

The `algol-iir-compiler` (v0.23.0) lowers `M[i, j]` to the row-major flat
index `(i − 1)*2 + (j − 1)` using `alloc_array`/`array_set`/`array_get` with
precomputed flat indices.  No backend change is needed for Llvm/Wasm/Jvm/Clr/Vm/Jit
— the IIR is identical to E5 1D arrays.  NativeAot on aarch64 required the
`aarch64-backend` v0.20.0 large-frame support (the 2D program produces 72
variable slots = 576 bytes, exceeding the old 504-byte limit).  Exit 42.


## 0.166.0 — 2026-07-02 — AL-multidim-real: ALGOL 60 2D **real** array matrix proof cell (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60 2D **real** (f64) array
running on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit).

```algol60
begin real array M[1:2, 1:2]; real sum; integer result;
  M[1, 1] := 10.25; M[1, 2] := 10.25;
  M[2, 1] := 10.75; M[2, 2] := 10.75;
  sum := M[1, 1] + M[1, 2] + M[2, 1] + M[2, 2];
  result := entier(sum) end
```

Same multidim flat-index lowering as the 0.165.0 integer cell, but the element
type is `real`: the fractional doubles ride the E5 8-byte slots, are summed with
the f64 `add` path to 42.0, then floored to an integer exit code via the E8
`entier` (`real_to_int_floor`) conversion.  Proves f64 multidim elements — the
follow-up flagged when AL-multidim first landed — with no backend change.
Exit 42.  Requires `algol-iir-compiler` 0.24.0.


## 0.170.0 — 2026-07-02 — AL-pow: ALGOL 60 exponentiation operator (all 7 backends)

**New matrix proof cell** (`lang_matrix.rs`): ALGOL 60's `↑` exponentiation
operator (spelled `^`) running on all 7 backends (NativeAot, Llvm, Wasm, Jvm,
Clr, Vm, Jit).

```algol60
begin integer result; result := 10 + 2 ^ 5 end
```

A nonnegative integer-literal exponent unrolls to repeated integer
multiplication and keeps the base's type, so `2 ^ 5` is the *integer* 32 —
exactly `mul`/`imul` the backends already run (no new IIR op). `↑` binds tighter
than `*`/`+`, so `10 + 2 ^ 5` = `10 + 32` = 42. The `real ↑ real` shape reuses
the `f64_pow` op BASIC's BA-pow already proved. Requires `algol-iir-compiler`
0.27.0. Exit 42.


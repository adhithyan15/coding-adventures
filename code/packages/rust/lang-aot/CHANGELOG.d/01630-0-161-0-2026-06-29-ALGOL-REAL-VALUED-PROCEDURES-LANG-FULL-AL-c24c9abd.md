## 0.161.0 — 2026-06-29 — ALGOL real-valued procedures (LANG-FULL AL13-real-proc)

One new matrix `Prog` cell proving ALGOL 60 `real procedure` declarations on all 7
backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- `real procedure scale(x); value x; real x; scale := x * 6.0; entier(scale(7.0))`
  — scale(7.0) = 42.0; entier(42.0) = 42 → Exit(42)

The call path (`real procedure` → `call` IIR instruction returning f64 → `entier`
standard function → `real_to_int_floor` → integer exit code) was already wired in
all 7 backends; this cell is the first explicit end-to-end matrix proof of it.

No new backend code was needed.  The `algol-iir-compiler` 0.21.0 bump adds the
corresponding VM-level unit test (`real_procedure_runs`).

856 total proven cells pass.


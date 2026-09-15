## 0.160.0 — 2026-06-29 — ALGOL recursive procedures, goto, nested block shadowing (LANG-FULL AL12)

Three new matrix `Prog` cells proving advanced ALGOL 60 control-flow and scoping
features on all 7 backends (NativeAot, Llvm, Wasm, Jvm, Clr, Vm, Jit):

- **Recursive procedure** — `fact(3)` using `n * fact(n - 1)` base case `n < 2`.
  Proves that backends correctly lower recursive `call`/`ret` pairs with separate
  stack frames.  Result = 6.  Exit 6.
- **`goto` loop** — accumulates `x := x + 7` until `x >= 42` via an unconditional
  label + conditional `goto`.  x: 7 → 14 → 21 → 28 → 35 → 42.  Exit 42.
- **Nested block variable shadowing** — three nested `begin … end` blocks each
  declare their own `integer x` and `boolean flag`, exercising the lexical scoping
  rules of ALGOL 60 §2.7.  Inner reads correct (innermost) binding; outer reads
  survive exit of inner block.  Trace: result = 31 + 10 + 1 = 42.  Exit 42.

855 total proven cells pass.


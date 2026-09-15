## 0.157.0 — 2026-06-29 — ALGOL 60 boolean variables + BASIC FOR STEP (LANG-FULL AL10 + BA-step)

Four new matrix `Prog` cells proved on all 7 backends (NativeAot, Llvm, Wasm,
Jvm, Clr, Vm, Jit):

### ALGOL 60 — boolean variables (AL10)

- `boolean b; b := true; if b then result := 42` — declared boolean variable
  used directly as an `if` condition; proves `bool`-typed `const`/`mov` + named
  variable condition reach every backend's branch op without a spurious coerce.
- `boolean b; b := false; if not b then result := 42` — unary `not` operator;
  LLVM `xor i1 %b, 1`, WASM `i32.eqz`, JVM/CLR integer NOT, VM `not` dispatch.
- `boolean a, b; a := true; b := false; if a and (not b) then result := 42` —
  two-operand `and` wired after a `not`-inverted sub-expression; proves compound
  boolean algebra end-to-end on all 7 backends.

### Dartmouth BASIC — FOR STEP > 1 (BA-step)

- `FOR I = 1 TO 5 STEP 2` + `LET S = S + I` + `NEXT I` → S = 1+3+5 = 9;
  `PRINT S` → `"9"`.  The `_for_<n>_step` IIR slot holds the compile-time `2`
  constant; NEXT adds it before re-testing `I <= 5`.  Distinguishes from the
  existing default-STEP-1 cell (which sums to 15), proving the STEP clause is
  live not dead on all 7 backends.

848 total proven cells pass.


## 0.4.0 — ALGOL 60 `real` arithmetic (LANG-FULL AL1 / enabler E3, phase 1)

`real` was rejected everywhere ("real scalars/parameters/literals on the common
slice"). It now lowers to the IIR `f64` type:

- **`real` type** (scalars, procedure parameters, procedure return types) →
  `ScalarType::Real` → IIR `f64`; a `real` slot is seeded to `0.0`.
- **`REAL_LIT`** (`3.14`, `1.0E-3`, `100E2`) parses via `f64::from_str` into an
  `Operand::Float`.
- **Arithmetic** `+` `-` `*` and **unary minus** accept `real` operands and emit
  the op with an `f64` `type_hint` (so the runtime computes in double); **`/`**
  is real division (also `f64`). `div`/`mod` remain integer-only (ALGOL's
  integer operators). **Ordered + equality comparisons** of reals compare at
  `f64` width (the operand-width hint, as for integers).
- **No implicit integer→real coercion** in this slice: mixing `integer` and
  `real` in one operator, or using `/` on integers, is a clean `Type` error
  (coercion needs an IIR int→float convert op the code-gen backends don't carry
  yet).

**Verified by RUNNING** on the VM and JIT (`lang-aot` `lang_matrix.rs`): real
multiply + equality fold → exit 42, real division + ordered comparison → exit 1.
10 new unit tests; the former `rejects_real_declarations_cleanly` test is
updated (`real_declarations_compile_to_f64`).

**Scope (E3 phase 1).** Reals run on the VM and JIT, which carry a tagged float
value model. The five code-gen backends don't execute f64 yet — `iir-to-{llvm,
wasm,jvm}` model every variable slot as a uniform `i64` (E3-codegen-slots) and
`iir-to-cil-bytecode` / the native backends reject `Operand::Float`
(E3-clr / E3-native). Those are tracked in `LANG-FULL-IMPLEMENTATION.md`.


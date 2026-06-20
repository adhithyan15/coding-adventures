# Changelog

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

## 0.3.0 — ALGOL 60 switches + conditional designators (LANG-FULL AL5)

- Lower **switch declarations** (`switch s := a1, a2, a3`) and the **computed
  goto** that uses them (`goto s[i]`). A switch records an ordered list of
  target labels; `goto s[i]` selects the i-th (1-based) target via a linear
  `index == k ? jmp Lk` chain. An out-of-range subscript matches no arm and
  falls through to the next statement (ALGOL leaves this undefined; treated as
  a no-op, the conventional implementation choice).
- Lower **conditional designational expressions** in `goto`
  (`goto if b then L1 else L2`), including nested/parenthesised designators —
  the branch is emitted with the portable `jmp_if_false` / `jmp` / `label`
  subset, recursing on the else-designator.
- **Fixed comparison lowering** — `cmp_*` now carries the **i64 operand width**,
  not the `bool` result width. Emitting `bool` made the LLVM backend compare two
  `i64` operands at 1-bit `i1` (`3 == 1` truncates both to `1` → wrongly equal)
  and produced invalid IR that `clang` rejected outright, so every ALGOL program
  with a comparison (`if`, `for … while`, switch index) was latently broken on
  the code-gen backends — it had simply never been exercised there (no ALGOL
  matrix program used a comparison until the switch's index test). This is the
  same width fix the BASIC BA0 work applied.
- Proven by **running**: `lang-aot`'s `lang_matrix.rs` executes a 3-element
  switch (`goto s[3]` ⇒ exit 49) across native / LLVM / WASM / JVM / CLR / VM /
  JIT — `s[3]` chosen because an i1-truncated compare would mis-select the first
  arm, so the cell guards the cmp fix. Unit tests cover each switch index, the
  out-of-range fall-through, both conditional-designator branches, the rejection
  paths (undeclared switch, non-integer index), and the cmp operand width.
- **Limits (follow-ups):** switch-list elements must be plain labels
  (conditional / nested-subscript elements rejected); switch declarations are
  not block-scope-shadowable (a flat per-compilation map, save/restored across
  procedure boundaries).

## 0.2.0 — ALGOL 60 typed procedures with value parameters (LANG-FULL AL3)

- Lower **typed (function) procedures with `value` parameters** to sibling
  `IIRFunction`s in the module. A heading like
  `integer procedure sq(x); value x; integer x; sq := x*x` becomes a function
  `sq(x: i64) -> i64`, and a call `sq(7)` (in expression or statement position)
  becomes an IIR `call` whose `srcs[0]` names the callee. Procedure signatures
  are registered in a pre-pass over each block, so a procedure may be called
  before it is textually declared and may call itself (recursion).
- Proven by **running**: `tests/lang_matrix.rs` in `lang-aot` executes
  `result := sq(7)` ⇒ exit `49` across native-AOT / LLVM / WASM / JVM / CLR /
  VM / JIT. Unit tests cover multi-parameter procedures, boolean procedures,
  recursion (factorial via an if-statement body), statement-position calls, and
  the rejection paths (void procedures, call-by-name parameters, arity and type
  mismatches).
- **Scope and limitations** (tracked as follow-ups): only typed procedures with
  `value` parameters are modelled. Proper (void) procedures are rejected — they
  have no observable effect on the current executable slice (no output op, no
  by-reference or enclosing-scope mutation), so admitting one would lower code
  no test could witness. Procedure bodies are lexically flat: they see their
  own value parameters but not enclosing-block variables (call-by-name /
  Jensen's device and non-local access are future work).

## 0.1.0

- Add an ALGOL 60 scalar frontend for the LANG VM Rust chain.
- Lower integer and boolean declarations, scalar assignments, integer arithmetic including `div`/`mod`, comparisons, if/else, compound statements, goto labels, and simple `for step until` loops to `interpreter_ir::IIRModule`.
- Prove the emitted IIR runs through `vm-core`, `jit-core`, `aot-core`, WebAssembly, JVM, CLR, BEAM, and LLVM backend paths.

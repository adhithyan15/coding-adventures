# Changelog

## 0.5.1 — Fix: `for`-loop guard compares at operand width (ALGOL `for` runs on LLVM)

The `for … step … until` loop-guard comparisons (the step-sign check and the
ascending/descending bound checks) were emitted with `type_hint = "bool"` — the
boolean *result* type — instead of the integer *operand* width. A code-gen
backend reads a comparison's `type_hint` as its operand type, so on **LLVM**
(`iir-to-llvm`'s `lower_cmp`) this produced the invalid `icmp i1 <i64>, <i64>`
that `clang` rejects, leaving ALGOL `for` loops un-runnable on LLVM (they worked
on VM/JIT/JVM/CLR, which infer operand types differently). The three guards now
carry `"i64"`, matching the regular relational path (which already tags the cmp
with `lhs.ty.iir()`).

Effect: ALGOL `for`-loop programs now compile to valid LLVM IR and **run via
`clang`**. The E5 sum-of-squares array `Prog` (two `for` loops over an array,
exit 55) — previously LLVM-deferred — now runs on the LLVM matrix column. New
regression test asserts the guards compare at `i64`, not `bool`.

## 0.5.0 — One-dimensional arrays (LANG-FULL enabler E5 / AL2)

Array declarations and subscripts were rejected ("array variables/subscripts").
They now lower to the IIR's E5 array primitive (`interpreter-ir` 0.7.0), which
`vm-core` 0.7.0 executes on a bounds-checked heap:

- **`integer array A[1:10]`** (and `real array`) → an `alloc_array` whose length
  is the **run-time** span `upper - lower + 1`. ALGOL's *dynamic* bounds
  (`array A[lo:hi]` with expression bounds) work because the bounds are emitted
  as ordinary integer expressions, not folded constants. The binding records the
  lower bound so subscripts can be translated.
- **`A[i]` in an expression** → `array_get`, with the index translated to the
  IIR's **0-based** form `i - lower` (ALGOL arrays are declared with an explicit,
  often 1-based, lower bound).
- **`A[i] := e`** → `array_set`, same index translation, with `e`'s type checked
  against the element type.
- **Bounds-checked by construction**: an out-of-range subscript traps at run time
  (`vm-core` returns a `VMError`, surfaced as `CompileError::Runtime`).
- A segment with several names (`integer array A, B[1:2]`) declares **distinct,
  non-aliasing** arrays sharing one set of bounds.

Scope (this slice): **1-D**, with `integer`/`real` element types. Multidimensional
arrays (`M[i, j]`), non-numeric element types, and arrays as procedure parameters
produce a clear "unsupported" message and are tracked as follow-up. Verified end
to end by 9 new unit tests (store/load round-trip, 1-based and non-unit lower
bounds, fill-and-sum in `for` loops, distinct-array segments, out-of-bounds trap,
scalar-subscript and 2-D rejections, real arrays) plus a `lang-aot` matrix `Prog`
that runs a sum-of-squares array program on **VM + JIT** (exit 55). The code-gen
backends lower the array ops in E5 PR-3 (managed) and PR-4 (static).

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

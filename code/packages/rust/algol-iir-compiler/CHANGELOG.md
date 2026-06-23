# Changelog

## 0.9.0 — 2026-06-22 — `sign` standard function (LANG-FULL AL8, PR-2)

The second ALGOL 60 standard function (§3.2.4), **`sign`**, building on the
`abs` machinery from 0.8.0.

- `sign(E)` is the *signum*: `+1` if `E > 0`, `-1` if `E < 0`, `0` if `E = 0`.
  Unlike `abs`, the **result is always `integer`** regardless of the operand's
  type — `sign(-2.5)` is the integer `-1` (no real→integer coercion needed at
  the use site).  The operand may be `integer` or `real`.
- It lowers to the nested conditional `if E > 0 then 1 else if E < 0 then -1
  else 0`: a `cmp_gt` then a `cmp_lt` against a typed zero (compared at the
  operand width), with three `i64` constants moved into one result slot — the
  same store-per-branch shape (no SSA phi) `abs` uses, so it **runs on all seven
  backends** (native-AOT/LLVM/WASM/JVM/CLR/VM/JIT).  `E` is evaluated once.
- Same name-based, overridable resolution as `abs`: a user `procedure sign`
  wins over the built-in.
- **Verified by RUNNING:** a `lang_matrix.rs` cell — `43 + sign(0 - 1)` ⇒ exit
  **42** (the negative branch) — executes on every backend; plus 8 inline tests
  (positive / negative / zero integer `sign`, positive / negative real `sign`
  yielding an integer, composition with `abs`, the user-override case, and the
  wrong-arity rejection).

`entier` (floor of a real → integer) needs a float-floor+convert that is not a
portable IIR op, and `sqrt`/`sin`/`cos`/… need a runtime math library on every
backend; those are later AL8 slices.

## 0.8.0 — 2026-06-22 — `abs` standard function (LANG-FULL AL8, PR-1)

ALGOL 60 *standard functions* (§3.2.4) are built into the language rather than
user-declared procedures.  This release adds the first one, **`abs`**.

- `abs(E)` yields the absolute value of `E`, preserving its numeric type
  (`integer`→`integer`, `real`→`real`).  It lowers inline to the value of
  `if E < 0 then -E else E`: a `cmp_lt` against a typed zero, then a
  `jmp_if_false` choosing between a negated (`0 - E`, i.e. `sub`/`fsub`) and a
  pass-through `mov` into a single result slot.  This is the same store-per-branch
  shape the conditional-expression lowering already runs on **all seven backends**
  (native-AOT/LLVM/WASM/JVM/CLR/VM/JIT) — no backend learns anything about `abs`;
  it is compare + branch + subtract in the shared IIR.  `E` is evaluated once.
- **Resolution is name-based and overridable.** A standard function has no
  `proc_sigs` entry, so a call resolves to the built-in only when the name is
  *not* a user-declared procedure — a program that redeclares `procedure abs`
  gets its own version, exactly as the Report permits.
- **No grammar change.** `abs(x)` already parses as a `proc_call`; only the
  IIR-compiler's call lowering changed.
- **Verified by RUNNING:** a new `lang_matrix.rs` cell — `result := abs(0 - 42)`
  ⇒ exit **42** — executes on every backend; plus 9 inline tests (negative /
  positive / zero / composed integer `abs`, negative / positive real `abs`, the
  lowers-to-branches-not-a-call structural check, the user-override case, and the
  wrong-arity rejection).

`sign`/`entier`/`sqrt`/`sin`/`cos`/… follow in later AL8 slices (the
transcendentals need a runtime math library on every backend; the pure-IIR
`abs`/`sign`/`entier` come first).

## 0.7.0 — 2026-06-22 — `own` variables: static lifetime (LANG-FULL AL6)

ALGOL 60's `own` declarations (`own integer n`) now lower to **module globals**,
reusing the E6 global substrate (`global_load`/`global_store`). An `own` variable
is allocated once and retains its value across every call of its enclosing
block/procedure (ALGOL 60 §5.2.5), which is exactly the semantics a module
global gives — it zero-inits at module load and persists.

- `declare_var` gained an `is_own` flag; a declaration is materialised as a
  global when `is_own || captured` (E6). The slot is already unique per scope
  (`__algol_s<N>_<name>`, the per-procedure `scope_counter` differs), so two
  procedures' `own n` map to **distinct** globals — they don't alias.
- **A global is no longer given a per-declaration `const` zero-init.** For an
  `own` variable inside a procedure that init would re-zero it on every call,
  destroying persistence; for an E6-captured block scalar it was a dead
  register write shadowing the global. Globals zero-init once at module load,
  so the `const` is both unnecessary and wrong for them. Plain (register)
  scalars keep their zero-init.
- Proven by **running** on all 7 backends (`lang_matrix.rs`): `bump(d)` adds `d`
  to its `own integer n`; `bump(1) + bump(1) + bump(1)` accumulates `1 + 2 + 3
  = 6` (a non-`own` local would give `1 + 1 + 1 = 3`). Plus unit tests: lowering
  to `global_load`/`global_store` with no re-init `const`, VM-run persistence,
  and two procedures' `own` staying independent.
- Requires `coding-adventures-algol-parser` 0.2.0 (the `[ "own" ] type
  ident_list` grammar rule).

## 0.6.0 — 2026-06-22 — procedures share enclosing-block scalars as globals (LANG-FULL E6 layer 1)

A procedure body may now read and write a scalar declared in an **enclosing
block** — the canonical typed module global.  Previously a procedure could touch
only its own value parameters; an enclosing-scope reference was out of reach
(`compile_procedure` installs a fresh, isolated scope).

### Added
- **E6 capture analysis.**  At each block, before any scalar is declared, a
  pre-pass (`collect_block_captures`) scans every procedure body for the names it
  references (minus that procedure's own parameters / result name).  A block
  scalar whose name lands in this set is materialised as a module **global**
  (`VarBinding::is_global`) instead of a register — its slot doubles as the
  global's name.
- **Typed global ops at the access sites.**  A read of a captured scalar lowers
  to `global_load "name"` (via the new `read_scalar` helper); a write lowers to
  `global_store "name", v` — in **both** the procedure and the enclosing block,
  so they share one cell.  These are the same IIR ops every backend now runs
  (VM/JIT/LLVM/JVM/CLR + BEAM/WASM/native).
- Declaration order inside a block is now: non-procedure declarations →
  procedure bodies → statements, so a captured global is declared before any
  procedure that injects it.  `compile_procedure` re-injects the visible global
  bindings into the procedure's fresh scope so its body resolves them.

### Verified
- A procedure sharing an enclosing `counter` with its block (`integer procedure
  add(x); … add := counter := counter + x; counter := 40; result := add(2)`)
  lowers `counter` to `global_load`/`global_store` in both functions and **runs
  on the VM ⇒ 42**.  72 tests; the existing suite is unchanged (plain scalars
  stay registers).

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

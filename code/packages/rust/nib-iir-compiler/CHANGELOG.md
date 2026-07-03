# Changelog — `nib-iir-compiler`

## 0.19.0 — 2026-06-27 — const/static expressions fold at compile time (LANG-FULL N10)

Top-level `const` and `static` initializers now accept deterministic
integer/boolean const-expressions:

```nib
const BASE: u8 = 6 * 7;
static counter: u8 = BASE + 0;
fn main() -> u8 { return counter; }
```

The compiler folds the initializer expressions before emitting function bodies.
`const` references still lower to literal `const` instructions, while folded
`static` initializers seed the shared E6 global storage at `main` entry. Calls
and non-const names remain rejected in initializer expressions.

The lang matrix proves the folded const-backed static reaches exit code `42` on
native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.

## 0.18.0 — 2026-06-27 — logical NOT lowers through truthiness branches (LANG-FULL N9)

Unary `!` now lowers to a portable boolean inversion:

```text
dest = const 1
jmp_if_false value, done
dest = const 0
done:
```

The old behavior passed the inner expression through unchanged, so `!(1 == 2)`
behaved like `1 == 2`. The lang matrix now proves:

```nib
fn main() -> u8 { if !(1 == 2) { return 42; } return 0; }
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.

## 0.17.0 — 2026-06-27 — mutable module statics lower to shared globals (LANG-FULL N8)

Adds the first executed Nib `static` slice:

- collects top-level `static NAME: type = integer-literal;` declarations,
- emits each initializer at the top of `main` as `const` + `global_store`,
- lowers unshadowed static reads to `global_load`, and
- lowers assignments to `global_store`.

This deliberately kept initializer support literal-only, matching the existing
Nib `const` boundary. Const/static expression folding is closed by 0.19.0; BCD
storage semantics and Intel-4004 RAM mapping remain explicit follow-ups.

The lang matrix proves a shared counter:

```nib
static counter: u8 = 40;
fn bump(step: u8) -> u8 { counter = counter + step; return counter; }
fn main() -> u8 { let a: u8 = bump(1); let b: u8 = bump(1); return counter; }
```

Expected exit code is `42` on native-AOT + LLVM + WASM + JVM + CLR + VM + JIT.

## 0.16.0 — 2026-06-16 — bitwise NOT (`~`) lowers to the IIR `not` op (LANG-FULL N3)

Unary `~` now lowers to the shared IIR `not` op (bitwise complement). Two fixes
were needed:

1. **`compile_unary` lowers `~` (was a silent no-op).** It previously *dropped* the
   operator and passed the operand through, so `~0` compiled to `0`. It now emits a
   `not` op carrying the **narrow result width** (`u8`/`u4`) of the unary node, so every
   backend masks it mod-2ⁿ: `~0u8 = 255` (`-1 & 0xFF`), `~15u4 = 0`. Without the width
   the `not` would yield the i64 all-ones (`-1`), not the type's complement. Logical `!`
   stays a passthrough (boolean lowering is a separate item).

2. **`compile_expr` no longer unwraps a `~x` as a transparent wrapper.** The
   single-child-wrapper passthrough counts only child *nodes* (tokens filtered), so a
   `unary_expr` of shape `[TILDE, operand]` looked like a one-child wrapper and was
   unwrapped — discarding the `~` before `compile_unary` ran. It now keeps a `unary_expr`
   that carries a leading operator token.

Runs on **all 7 backends** (native/LLVM/WASM/JVM/CLR/VM/JIT), proven by executed
`lang_matrix.rs` programs (`~0u8 == 255`, `~15u4 == 0`). This was the last deferred Nib
N3 piece — it had waited on `iir-to-llvm` 0.12.0 (which grew the `not` op) and surfaced a
matching gap in `iir-to-cil-bytecode`'s textual emitter (0.21.0). New unit tests
`compiles_bitwise_not_with_narrow_hint` and `double_bitwise_not_is_identity`.

## 0.15.0 — 2026-06-16 — wrapping (`+%`) and saturating (`+?`) add (LANG-FULL N7)

Lowers Nib's two explicit-overflow additive operators (the grammar already
parsed them; the compiler now compiles them). Both are E2-unblocked — they
depend on the narrow-width register-wrap masking shipped in N6.

- **`+%` — wrapping add.** Maps to the same IIR `add` as `+`, carrying the
  narrow `type_hint` so the E2 backend mask wraps it: `15u4 +% 1` → `16 & 0xF =
  0`, `200u8 +% 100` → `44`. (`cir_op_for` gains `WRAP_ADD → "add"`.) Under E2 a
  plain `+` on a narrow type already wraps; `+%` makes the intent explicit.

- **`+?` — saturating add.** NOT a single op: `compile_binary_chain` lowers it to
  a **wide** `add` (i64, *unmasked* — so the true total is visible) followed by a
  clamp branch: `const MAX` (15 for u4, 255 for u8 from the node's inferred type),
  `cmp_gt sum, MAX`, then `mov dest, sum` / `jmp_if_false` / `mov dest, MAX` /
  `label` — i.e. `dest = min(sum, MAX)`. So `15u4 +? 1` → `15`, `200u8 +? 100` →
  `255`, and a non-overflowing `3 +? 4` → `7`.

Verified by RUNNING on vm-core (`tests/n7_check.rs`) and across **all 7 backends**
(native/LLVM/WASM/JVM/CLR/VM/JIT) via new `lang-aot` matrix programs (comparison-
based, so they distinguish a saturated `255` / wrapped `44` from the unwrapped
`300`). No grammar or `nib-type-checker` change (the additive operators are
type-inferred from their operands).

## 0.14.0 — 2026-06-16 — narrow `type_hint`s on arithmetic (LANG-FULL E2 / N6)

Activates the LANG-FULL E2 integer-width-and-wrap semantics in the Nib frontend:
the final, frontend-wiring step of the E2 integration. (Wiring this up surfaced
that three of the seven backends couldn't yet consume a narrow op the way a real
frontend emits it; those were fixed first — iir-to-wasm grew an i64 register
model, iir-to-jvm uses the int model atop its `concretize`-to-i32 pass, and
iir-to-cil was verified int32-uniform. The other four already masked.)

### Changes

**Narrow `type_hint`s on arithmetic / bitwise ops (`compile_binary_chain`)**

Previously every IIR `add`/`sub`/`mul`/`div`/`and`/`or`/`xor` instruction was
emitted with `type_hint = "i64"`, so backends could not distinguish a 64-bit add
from a u8 add and never masked the result. Now:

- Each arithmetic/bitwise binary op looks up the `nib-type-checker` 0.3.0
  annotation on the chain node via `lookup_node_type(node, types)`.
- `U8` → `"u8"`, `U4` → `"u4"`, anything else (or unannotated) → `"i64"`.
- **Comparison ops** (`cmp_eq`, `cmp_ne`, `cmp_lt`, etc.) are deliberately
  excluded from narrowing: they operate on wide operands and emit a `bool`
  result; the `i64` hint keeps the LLVM backend from emitting invalid `icmp` on
  narrowed operands.

Consts/`let`s/`ret`/calls stay `i64` (`nib_ty_str`); the narrow width lives only
on the arithmetic op, which every backend masks to width. **Unary `~` (N3) is
deferred** — it lowers to an IIR `not` op, which the LLVM backend does not yet
support; `compile_unary` still passes the inner expression through.

### Verified

`lang-aot/tests/lang_matrix.rs` gains two new N6 executed programs:

1. **Wrap proof**: `fn main() -> u8 { let x: u8 = 200 + 100; if x == 44 { return 1; } return 0; }` → exit **1**
   on native/LLVM/WASM/JVM/CLR/VM/JIT.  The comparison (`x == 44`) proves the
   add wrapped *before* the comparison, not just that the exit-code low byte
   happens to match.

2. **Magnitude regression guard**: `fn main() -> u8 { return 6 * 7; }` → exit **42**
   on all backends.  Without bidirectional typing (`nib-type-checker` 0.3.0), `6`
   and `7` infer as `u4` (magnitude ≤ 15), mask `6 * 7 = 42` to `42 & 0xF = 10`,
   and the test would fail.  Passing proves `6` and `7` adopt the `u8` return
   context and the product is left intact.

## 0.13.0 — 2026-06-13 — module-scoped `const` declarations (LANG-FULL N5)

Adds Nib's top-level `const NAME: type = literal;`. Previously `const_decl` (and
`static_decl`) were silently dropped by `function_nodes`, so referencing a const
produced a dangling variable.

- New `collect_consts` gathers module-scoped consts (they are `top_decl`, like
  `fn`) into a `name → i64` map before any function is compiled, folding each
  value expression to an `i64`. `compile_program` populates `Compiler.consts`.
- `compile_primary` resolves a const reference to a fresh `const` instruction
  with the const's value — a compile-time fold, so consts need **no runtime
  storage** and run on every backend with no per-backend work. A `let`/parameter
  of the same name **shadows** the const (the fold only fires when the name isn't
  a local in scope).
- V1 folded **integer-literal** consts (`INT_LIT`/`HEX_LIT`); a non-literal value
  (`const N = 6 * 7;`) was a clear error rather than a silent miscompile.
  Const-expression folding is closed by 0.19.0.

Verified by RUNNING on every backend: `lang-aot/tests/lang_matrix.rs` gains
`const N: u8 = 42; … return N;` → 42 and `const A = 30; const B = 12; … A + B`
→ 42, across native/LLVM/WASM/JVM/CLR/VM/JIT. New unit tests
`const_reference_folds_to_its_literal`, `multiple_consts_in_arithmetic`,
`non_literal_const_is_rejected`.

`static` declarations remain deferred (mutable module state is a larger,
backend-touching item).

## 0.12.0 — 2026-06-13 — short-circuit `&&` / `||` (LANG-FULL N4)

Adds Nib's logical `&&` / `||`. Unlike the other operators these cannot go through
`compile_binary_chain` (which evaluates both sides eagerly and has no `cir_op_for`
mapping for `LAND`/`LOR`) — they must **short-circuit**: the right operand is
evaluated only when the left does not already decide the result.

New `compile_short_circuit` lowers an `and_expr`/`or_expr` to a result slot guarded
by branches, using only `jmp_if_false` / `jmp` / `label` (the portable subset every
backend lowers — the CLR textual `.il` path has no `jmp_if_true`):

```text
// a && b              // a || b
mov r = a              mov r = a
jmp_if_false r, end    jmp_if_false r, eval_b
mov r = b              jmp end
label end              label eval_b ; mov r = b ; label end
```

Chains fold left-to-right; `r` is the `dest` of 2+ `mov`s so every backend promotes
it to a stack slot automatically.

Verified by RUNNING on every backend — `lang-aot/tests/lang_matrix.rs` gains, across
native/LLVM/WASM/JVM/CLR/VM/JIT:
- a `&&` short-circuit **proof**: `1 == 2 && 84 / 0 == 0` returns 7 (not 9, not a
  crash) — the divide-by-zero RHS is positive proof it was never evaluated;
- a `||` short-circuit proof: `1 == 1 || 84 / 0 == 0` returns 7;
- a `&&` true-path program → 1.

New unit tests `logical_and_short_circuits`, `logical_or_short_circuits` (assert the
right operand's compare is emitted *after* the short-circuit guard).

## 0.11.0 — 2026-06-13 — bitwise `&` `|` `^` (LANG-FULL N3)

Adds Nib's binary bitwise operators. The grammar's `bitwise_expr` level already
produced `AMP`/`PIPE`/`CARET` nodes routed through `compile_binary_chain`, so this
is a `cir_op_for`-only change: `&` → `and`, `|` → `or`, `^` → `xor` (the shared IIR
ops every backend implements).

Verified by RUNNING on every backend: `lang-aot/tests/lang_matrix.rs` gains
`12 & 10` → 8, `12 | 3` → 15, `6 ^ 5` → 3, executed across
native/LLVM/WASM/JVM/CLR/VM/JIT. (The CLR textual `.il` path was missing these
opcodes — fixed in `iir-to-cil-bytecode` 0.19.0, surfaced by the executed test.)
New unit test `compiles_bitwise_and_or_xor`.

Unary `~` (bitwise NOT) is still deferred: a correct result needs to mask to the
declared width (`~x` on a `u8` flips 8 bits, not the full 64-bit register), which
depends on the integer-wrap enabler **E2**.

## 0.10.0 — 2026-06-13 — for loops (LANG-FULL N2)

`compile_stmt` no longer returns `Unsupported("stmt: for_stmt")` — Nib's
`for NAME: type in lo .. hi block` now compiles. The grammar and parser already
produced `for_stmt` nodes; this adds the lowering.

`compile_for` desugars to the same canonical loop shape `compile_while` uses,
reusing the existing `mul_expr`/`add`/`cmp_lt`/label machinery:

```text
mov  i = lo            ; bounds evaluated once at loop entry
<eval hi → h>
label for_<n>_top
cmp_lt c = i, h        ; range is EXCLUSIVE of hi (`1 .. 6` ⇒ i = 1,2,3,4,5)
jmp_if_false c, for_<n>_end
<body>
add  t = i, 1 ; mov i = t
jmp for_<n>_top
label for_<n>_end
```

Everything flows through `i64` slots, so the loop-counter reassignment is the
same shape every backend already lowers for Brainfuck's pointer increment.
Nested loops get distinct labels via `fresh_label`.

Verified by RUNNING on every backend: `lang-aot/tests/lang_matrix.rs` gains a
sum-loop (`for i in 1..6 { s += i }` → 15, using the loop variable) and a nested
loop (3 × 2 → 6), executed across native/LLVM/WASM/JVM/CLR/VM/JIT. New unit
tests: `compiles_for_loop`, `nested_for_loops_get_distinct_labels`.

Known limitation (out of scope, a backend concern not a frontend one):
reassigning a **function parameter** inside a loop produces invalid LLVM IR
(the IIR-to-LLVM backend allocas locals but keeps params in SSA). The for-loop
idiom uses a `let` local accumulator, which works everywhere.

## 0.9.0 — 2026-06-13 — multiplication and division (LANG-FULL N1)

Adds `*` and `/` to Nib. The Intel-4004 has no multiply/divide instruction, so
these were reserved tokens in v1; they now lower to the shared IIR `mul` / `div`
ops, which every general backend (VM / JIT / native / LLVM / WASM / JVM / CLR)
implements directly.

- New grammar level `mul_expr = bitwise_expr { ( STAR | SLASH ) bitwise_expr }`,
  slotted between `add_expr` and `bitwise_expr` so `*`/`/` bind tighter than
  `+`/`-` (`2 + 3 * 4` = `2 + (3*4)` = 14) and are left-associative. Parser
  regenerated.
- `cir_op_for`: `STAR` → `mul`, `SLASH` → `div` (typed CIR mnemonics, not
  `call_builtin "*"`, so the IIR-to-* backends accept them).
- `compile_expr` routes the new `mul_expr` node through the generic
  `compile_binary_chain`; `is_expr_rule` recognises it.

Verified by RUNNING on every backend: `lang-aot/tests/lang_matrix.rs` gains
`6 * 7` → 42 and `84 / 2` → 42, executed across native/LLVM/WASM/JVM/CLR/VM/JIT.
New unit tests: `compiles_multiplication`, `compiles_division`,
`multiplication_binds_tighter_than_addition`.

## 0.8.0 — 2026-06-11 — finish the i64 materialization (const literals + call results) (LANG-MATRIX LM-W Nib)

Completes the integer-type materialization started in 0.7.0. That release fixed
`widen_nib_type` (function signatures / `let` types) but left **`nib_ty_str`** — which
types const literals, `ret` values, and call results — emitting the narrow `"u8"`, and
the const-emit path's *fallback* (for an integer literal the type-checker didn't
annotate) was hard-coded `"u8"`. So a bare literal argument like `double(21)` emitted a
`u8` (→ `i32`) const into an `i64` parameter. The LLVM backend tolerated this (its call
site uses the callee's parameter type), but the **strict WASM backend trapped**:
`type mismatch: expected i64, got I32(21)`.

Both `nib_ty_str` (`u4`/`u8`/`bcd` → `i64`) and the un-annotated-literal fallback
(`"u8"` → `"i64"`) now materialise integers as `i64`, so the whole Nib IIR is uniformly
`i64` for integers — const literals, `let`s, arithmetic, `ret`, calls, and signatures
all agree. Verified by RUNNING: Nib `double(21)` → 42 on WASM (previously a trap), still
→ 42 on LLVM and native AOT (no regression); all nib-iir-compiler (28),
`iir-to-wasm` (46), and `iir-to-llvm` (102) tests green. Narrow semantic width remains a
deferred backend-masking concern.

## 0.7.0 — 2026-06-11 — materialise integer types to i64 uniformly (LANG-MATRIX LM-L Nib)

Fixes a type-**inconsistency** in the emitted IIR that a strict backend (`iir-to-llvm`)
rejected. The frontend's instruction bodies already use `i64` for integers
(`compile_binary_chain` and the `ret` default both emit `"i64"`; the long-standing
intent is "Nib's u4/u8/bool all materialise as i64 at the IIR level"), but
`extract_params` / `extract_return_type` (and `let` types) left the **function
signature** as the narrow `u8`. So a function compiled to `define i8 @double(i8 %x)`
with an `add i64 %x, %x` body — `'%x' defined with type 'i8' but expected 'i64'`.

`widen_nib_type` now maps the integer types (`u4`/`u8`/`bcd`) to `i64`, completing the
frontend's own convention so signature and bodies agree (`bool`/`void` unchanged). The
IIR is now uniformly `i64` for integers — matching the instruction convention, the
native-AOT machine-word model, and McCarthy's lowering. Verified by RUNNING: Nib on
LLVM → 42 (previously a clang type error), Nib on native still → 42 (no regression);
all nib-iir-compiler unit tests and the `iir-to-llvm` backend tests stay green. The
narrow semantic width (u4/u8 wraparound) remains a backend-masking concern, deferred.

## 0.6.0 — 2026-05-30 (NIB06 — source-location threading for debugger)

### Added — Real source positions in `IIRFunction.source_map`

Nib's emitted IIR now carries real `(line, column)` per instruction
in `IIRFunction.source_map`, in lockstep with `instructions`.
Previously the field was filled with `SourceLoc::SYNTHETIC` (matching
the old `vec![SYNTHETIC; instructions.len()]` placeholder).

This is the prerequisite for line-based breakpoints in the future
`nib-dap` debugger crate.  Without real positions, the debug
sidecar built by the DAP layer cannot resolve `setBreakpoints
{ file, lines: [N] }` requests to IIR instructions.

This mirrors the pattern landed for `oct-iir-compiler` 0.4.0
(OCT05 / PR #4583) and `dartmouth-basic-iir-compiler` 0.4.0
(BASIC05 / PR #4587).  The Nib compiler was structurally different
from those two — it threads `out: &mut Vec<IIRInstr>` through every
helper rather than centralising on a single `emit` site — so we
introduced an `emit_to(out, instr)` wrapper that every IIR-emitting
call now funnels through, preserving the lockstep invariant.

### Implementation

- New `node_loc(&GrammarASTNode) -> SourceLoc` helper extracts
  `(start_line, start_column)` from an AST node, falling back to
  `SYNTHETIC` when the parser couldn't attach positions.
- `Compiler` gained two fields: `source_map: Vec<SourceLoc>` (the
  per-function accumulator) and `current_loc: Cell<SourceLoc>`
  (the "currently compiling" position).  Manual `impl Default`
  replaces the `#[derive(Default)]` since the `Cell` initial state
  benefits from being explicit (SYNTHETIC).
- New `Compiler::emit_to(&mut self, out, instr)` wrapper that pushes
  to both the function body and `source_map` simultaneously.  Every
  IIR-emitting call site in the module (16 of them, spanning
  `compile_let` / `compile_if` / `compile_while` / `compile_call_expr`
  / `compile_binary_chain` / `compile_primary` / the trailing
  defensive `ret_void`) now funnels through this helper.
- `compile_stmt` calls `set_loc(node_loc(stmt))` on entry — all
  instructions emitted for that statement (label + body) inherit the
  statement's source line, including those from compiled
  sub-expressions.
- `compile_function` resets `source_map` and sets the initial loc to
  the fn declaration's own position (so any pre-stmt emissions get a
  sensible source line rather than SYNTHETIC).
- `compile_function` ends with the move-with-defensive-padding shape:
  `iir_fn.source_map = std::mem::take(&mut self.source_map)` after
  ensuring lockstep — the same shape Oct + BASIC use.

### Tests

- 2 new unit tests:
  - `source_map_lockstep_with_instructions`: every function's
    `source_map.len() == instructions.len()`.
  - `source_map_carries_real_line_numbers`: a 4-line Nib program
    (fn header, two let stmts, return) produces entries for each
    non-fn-decl line.
- All existing lib tests still pass.

## 0.5.0 — 2026-05-28 (NIB05 — JIT via GenericCirJit)

### Added — Nib programs JIT-compile via `jit-core::GenericCirJit`

With `jit-core::GenericCirJit` landed (jit-core 0.3.0) and Oct's
integration validated (PR #4555), Nib gets a real JIT without a
per-language Backend impl.  Nib is the **third** language to plug
into the JIT chain (after Brainfuck and Dartmouth BASIC) and the
**second** to do so via `GenericCirJit` directly — proving the
architectural pattern.

### Changed — `let` and `assign` emit typed `mov` instead of `call_builtin "_move"`

Previously, Nib's `assign_stmt` and `let_stmt` emitted
`call_builtin "_move"` with `srcs = [Var("_move"), Var(rhs)]`.
This was the historical pre-Path-A form that AOT specialised to
typed CIR, but `vm-core` and `GenericCirJit` reject unknown builtin
names (`_move` isn't a registered runtime builtin — it's a
compile-time marker).

The new emission is the canonical typed form:

    mov name <- rhs [type]

which `vm-core`'s dispatch table handles directly and
`GenericCirJit::compile()` translates to a `MOV` bytecode opcode.
The AOT backends already accept typed `mov` (twig-vm 0.19.0+'s
dispatch wrapper, iir-to-* validators in their respective 0.4.x+).

### Changed — `IIRFunction::type_status = FullyTyped` override

`IIRFunction::new`'s automatic `infer_type_status` returns
`PartiallyTyped` because Nib's control-flow ops (`label`, `jmp`,
`jmp_if_false`, `ret_void`) carry `"void"` hints, and `"void"` is
NOT in `interpreter_ir::opcodes::CONCRETE_TYPES`.  Every Nib
instruction is in fact statically known (no `"any"` hints), so the
function is genuinely fully typed for the JIT's threshold-zero
compile path.  Mirrors Brainfuck, BASIC, and Oct.

### Tests

- 4 new end-to-end tests in `tests/jit_e2e.rs`:
  - `nib_jit_returns_constant_42`: `fn main() -> u8 { return 42; }`
  - `nib_jit_inline_arithmetic`: `return 30 + 12;` → 42
  - `nib_jit_let_and_add`: `let x: u4 = 7; return x;` → 7
  - `nib_jit_if_else`: `if 1 == 1 { return 100; } else ...` → 100
- All 11 existing lib tests continue to pass.
- Downstream `lang-aot` (8 + 11) continues to pass.

## 0.4.0 — 2026-05-22 (typed CIR ops — unblocks IIR-to-* backends)

Nib's IIR output is now accepted by every IIR-to-* backend
(`iir-to-wasm`, `iir-to-jvm-class-file`, `iir-to-cil-bytecode`,
`iir-to-beam`) without needing the AOT pipeline's `pre_lower_aot_builtins`
pre-pass.

### Background

A 5-frontend × 4-backend probe matrix run after the vm-core `mov`
dispatch fix (PR #3888) showed that Nib's IIR output was rejected by
every IIR-to-* backend with the same three errors:

```text
UntypedInstruction: function "main", op "call_builtin" has type_hint "any"
UnsupportedOp:      function "main", op "call_builtin" is not supported …
UntypedInstruction: function "main", op "ret" has type_hint "any"
```

Root cause: `compile_binary_chain` emitted `call_builtin "<op>"` with
`type_hint: "any"`, expecting the AOT chain's `pre_lower_aot_builtins`
pass to rewrite each one to a typed CIR op (`add`, `cmp_eq`, …) before
the native backends saw it.  That assumption held for `lang-aot`'s AOT
path but broke every IIR-to-* backend (they validate against concrete
CIR opcodes).

### Fix

Mirror `oct-iir-compiler::compile_binary` exactly:

- `compile_binary_chain` now emits typed CIR mnemonics directly:
  `+` → `add`, `-` → `sub`, `==` → `cmp_eq`, `!=` → `cmp_ne`,
  `<` → `cmp_lt`, `>` → `cmp_gt`, `<=` → `cmp_le`, `>=` → `cmp_ge`.
- `type_hint` is now `"i64"` for every binary op (Nib's narrow types
  u4/u8/bool all flow through 64-bit slots at the IIR level — the
  function's declared Nib return type stays on `IIRFunction`).
- `return_stmt`'s fallback type_hint is now `"i64"` instead of
  `"any"` (which leaked through to the IIR-to-* validators).

### What still works

The AOT chain (`lang-aot`) is unchanged.  `pre_lower_aot_builtins`
becomes a no-op when there's nothing to rewrite, so the typed `add` /
`cmp_*` ops flow straight through.  All five existing lang-aot Nib
e2e tests still pass:

- `end_to_end_nib_returns_42_via_lang_aot`
- `end_to_end_nib_arithmetic_via_lang_aot`
- `end_to_end_nib_cross_fn_call_returns_42`
- `end_to_end_nib_print_writes_42`
- `end_to_end_nib_while_loop_counts_to_10`

### Tests added (5)

`tests/backend_compat.rs`:

- `nib_iir_accepted_by_iir_to_wasm`
- `nib_iir_accepted_by_iir_to_jvm`
- `nib_iir_accepted_by_iir_to_clr`
- `nib_iir_accepted_by_iir_to_beam`
- `nib_iir_with_comparison_accepted_by_every_backend` — exercises
  `cmp_lt` (verifies the comparison-operator mapping too).

Plus an updated `tests::compiles_arithmetic` unit test that explicitly
guards against the old `call_builtin "+"` shape ever leaking back in.

### Helper rename

`builtin_for(text, type_name) -> Option<&str>` → `cir_op_for(text, type_name)`.
Same callers; the return value is now a typed CIR mnemonic instead of
the bare operator symbol.

## 0.3.0 — 2026-05-20 (NIB04 step 3 — while loops)

Closes the third (and final V1) NIB04 step.  Nib programs can now use
`while expr block` for unbounded condition-driven iteration.

**New `compile_while`.**  Lowers to the canonical IIR loop shape both
backends already support:

```text
label  while_<n>_top
<eval cond → c>
jmp_if_false c, while_<n>_end
<body>
jmp while_<n>_top
label  while_<n>_end
```

The guard is re-evaluated each iteration; body mutations use
`assign_stmt` which already goes through `call_builtin "_move"` to
update the slot in place.

**Grammar / lexer changes** (in sibling crates):
- `nib.grammar` adds `while_stmt = "while" expr block;` and an entry
  in the `stmt` alternation.
- `nib.tokens` adds `while` to the keyword set.
- `nib-parser/_grammar.rs` and `nib-lexer/_grammar.rs` regenerated by
  hand to match (the auto-generation tool is library-only).

**Tests added (2 unit + 1 e2e):**
- `compiles_while_loop` — IIR shape contains label / jmp_if_false /
  jmp / label.
- `compiles_while_with_nested_call` — `while n < 3 { n = n + one(); }`
  emits both `call` and `jmp_if_false`.
- `end_to_end_nib_while_loop_counts_to_10` (in lang-aot) —
  hand-built Nib program with `let n: u4 = 0; while n < 10 { n = n + 1; }
  return n;` compiles + links + runs + exits with code 10.  Verified
  locally on Windows.

## 0.2.0 — 2026-05-20 (NIB04 — print + cross-function calls)

Adds proper lowering for `call_expr` nodes in the Nib AST.  Before
this release the IIR compiler silently treated `foo(args)` as a bare
variable reference to `foo`, dropping every argument and emitting a
broken `ret Var("foo")` — which compiled to "return the function's
own address" at the AOT layer.  Two of NIB04's three V1 steps land
here:

**1. `print(x)` lowers to `call_builtin "print_i64", x`.**  The
runtime helper `__twig_print_i64` already exists from LANG75; no new
runtime work needed.  V1 `print` takes exactly one i64-shaped
argument; zero or two arguments produces a clean `Unsupported` error.

**2. Cross-function calls.**  `f(a, b, c)` lowers to a proper
`call f, a, b, c -> dest` IIR instruction.  The x86_64 + aarch64
backends already implement cross-function relocations (LANG43 PR
#3331); this PR just wires up the frontend.

**3. Zero-argument calls.**  `f()` works too — emits `call f -> dest`
with `srcs.len() == 1` (just the callee).

**Step 3 of NIB04 (while loops) is deferred** to a follow-up PR
because it requires grammar changes (no `while` rule today) plus
regenerating `nib-parser/src/_grammar.rs`.

### Tests added (4)

- `compiles_print_call` — IIR shape check for `print(42)`.
- `compiles_cross_function_call` — `double(21)` from main produces
  `call double, 21 -> dest`.
- `compiles_zero_arg_call` — `forty_two()` produces `call forty_two`
  with no extra srcs.
- `rejects_print_with_wrong_arity` — `print()` with 0 args fails
  with `Unsupported` instead of silently producing garbage IIR.

## 0.1.0 — 2026-05-05

Initial release.  Compiles Nib source to `interpreter_ir::IIRModule`,
unlocking the LANG-runtime AOT (and JIT, eventually) pipeline for the
Nib language.

### Coverage

- `fn name(params...) -> ret_ty { body }` → `IIRFunction`
- `let name: ty = expr;` → `const + _move`
- `return expr;` → `ret`
- Integer literals (`5`, `0x1F`)
- Identifier references / parameters
- Binary arithmetic (`+`, `-`) → `call_builtin "+"` etc.
  (lowered to typed CIR by `aot-core::specialise`)
- Comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`) — same lowering
- `if expr { ... } else { ... }`

### Out of scope (deferred)

- Cross-function calls (V1 aarch64-backend has no relocation support yet)
- Wrap/saturating arithmetic, bitwise ops
- For loops over ranges
- BCD operations

### End-to-end demonstration

Six Nib programs compile through `nib-iir-compiler` →
`twig-aot::compile_module_macos_arm64_object` → `ld` → runnable
ARM64 Mach-O on Apple Silicon.  Each program's exit code matches its
intended return value:

| Source | Exit |
|---|---|
| `fn main() -> u4 { return 9; }` | 9 |
| `fn main() -> u4 { return 3 + 4; }` | 7 |
| `fn main() -> u4 { let x: u4 = 5; return x; }` | 5 |
| `fn main() -> u4 { if 1 == 1 { return 4; } else { return 9; } }` | 4 |
| `fn main() -> u4 { if 1 == 2 { return 4; } else { return 9; } }` | 9 |
| `fn main() -> u4 { if 3 < 5 { return 1; } else { return 0; } }` | 1 |

This validates the path to deprecating the older `compiler-ir::IrProgram`
chain: any language with a frontend → IIR shim now inherits the full
AOT (and forthcoming JIT) infrastructure.

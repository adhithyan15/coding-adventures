# Changelog — `nib-iir-compiler`

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

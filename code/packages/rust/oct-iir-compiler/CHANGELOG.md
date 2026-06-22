# Changelog — `oct-iir-compiler`

## 0.8.0 — 2026-06-22 — `static` module globals + void calls (LANG-FULL O3)

Top-level `static` declarations were collected by the type checker but **silently
dropped** at IIR-gen — Oct programs could only use function-local registers. They now
lower to the IIR module-global ops (`global_load` / `global_store`, LANG32 — the same
path ALGOL's enclosing-block scalars use for E6 globals), so a `static` is shared across
every function and survives across calls.

- **`static counter: u8 = 40;`** → the initialiser runs once at the top of `main`
  (`const 40; global_store "counter", …`), and every read/write of the name routes
  through `global_load`/`global_store` instead of a register. A `let` local with a
  different name is unaffected; the read site only treats *declared statics* as globals.
- Proven by **running** on all 7 backends (`lang_matrix.rs`): a `static counter`
  initialised to 40, incremented twice by a *separate* `bump()` function, printed via
  `out` → `42`. A per-function register model would print `40`; getting `42` proves the
  value lives in one shared module global. Also a JIT e2e test asserts the same shared
  mutable global accumulates to 42 across `run()`/`bump()`.
- **Void-call fix (latent bug surfaced by the O3 proof's `bump()`):** a call to a
  void-returning user function now emits the IIR `call` with **no `dest`**. Previously it
  always bound a result register (`%t = call void @f()`), which is malformed LLVM
  ("instructions returning void cannot have a name"). Every prior Oct program only ever
  called the non-void `side()`, so this never fired. A `void_fns` pre-pass records each
  void function (forward references work) and `compile_call_expr` consults it.
- **Limitation:** a body-level `static` (a static-lifetime *local*, ALGOL-`own`-like) is
  still ignored — only top-level statics lower. A local `let` may not shadow a static.

## 0.7.0 — 2026-06-16 — u8 width & wrap: bitwise `~` and wrapping arithmetic (LANG-FULL O2)

Oct's only integer type is `u8` (the 8008 byte; `bool` is the only other type), and the
language spec says arithmetic "wraps modulo 256". Until now every value collapsed to an
unmasked `i64`, so `200 + 100` produced `300` and `~0` produced `-1`. Now:

- **Arithmetic / bitwise ops carry the `u8` type_hint** (`add`/`sub`/`and`/`or`/`xor`), so
  every backend masks the result mod-2⁸: `200 + 100 = 44`. A **comparison** (`cmp_*`) keeps
  the `i64` hint — its 0/1 `bool` result must not be masked and its operands ride i64 slots.
- **Unary `~` lowers to the `not` op with the `u8` hint** (it already emitted `not`, but at
  `i64` width — flipping all 64 bits). Now `~0 = 255`. Logical `!` is unchanged (still a
  bare `not`; proper boolean negation is a separate item — only `~` is in O2 scope).

There is no width to *track* the way Nib does — Oct has exactly one integer width, so every
integer op is `u8` by construction. Proven by RUNNING on all 7 backends (`out(1, ~0)` → 255,
`out(1, 200 + 100)` → 44). Needs `iir-to-jvm-class-file` 0.14.0 (Oct's printing programs keep
the JVM long model, where the narrow mask had to become `i2l; land`). New unit tests
`o2_arithmetic_carries_u8_hint_so_it_wraps`, `o2_bitwise_not_carries_u8_hint`.

## 0.6.0 — 2026-06-13 — short-circuit `&&` / `||` + i64 function returns (LANG-FULL O1)

### `&&` / `||` now short-circuit

They were lowered as **eager** bitwise `and`/`or` (both sides always evaluated).
That is observably wrong once an operand has a side effect — in Oct, a comparison
whose side is a function call that `out`-puts (`f() == 1`): the call ran
unconditionally. `compile_expr` now routes `and_expr`/`or_expr` to a new
`compile_short_circuit` that builds a result slot guarded by `jmp_if_false` / `jmp`
/ `label` (the portable subset — the CLR textual path has no `jmp_if_true`); the
right operand is evaluated only when the left doesn't decide the result.
Single-operand `and_expr`/`or_expr` (no real operator) pass through to
`compile_binary`. The eager `LAND`/`LOR` arms are removed.

This is now **verifiable by running** thanks to O-OUT: `lang-aot`'s `lang_matrix`
runs `if 1 == 2 && side() == 1 { … } else { out(1, 9) }` where `side()` prints 5 —
correct short-circuit prints just `9`; the old eager code printed `5` then `9`. So
stdout `"9"` (and `"7"` for the `||` case) is positive proof the RHS was skipped.

### Fixed — a non-void function's return type materialises as `i64`

The proof needs a `side() -> u8` helper, which surfaced a latent bug: Oct put the
**declared** return type (`u8`) on the `IIRFunction`, but params and the body
already flow as `i64` — so the IIR-to-LLVM backend emitted `define i8 @side()` and
the `ret` of an i64 value failed (`value doesn't match function result type 'i8'`).
A non-void return now materialises as `i64`, matching the params (already widened)
and the body — the same convention Nib uses.

New unit tests: `logical_and_short_circuits`, `logical_or_short_circuits`,
`typed_function_return_materialises_as_i64`.

## 0.5.0 — 2026-06-13 — the `out` intrinsic prints to stdout (LANG-FULL O-OUT)

`compile_intrinsic` previously rejected **all** Intel-8008 intrinsics. It now wires
`out(port, value)` to stdout: the 8008 writes `value` to I/O `port`; on the general
LANG backends all 24 output ports collapse to **stdout**, lowered as
`call_builtin "print_i64"` — the same print builtin Dartmouth BASIC's `PRINT` uses,
already supported on every backend (VM/JIT, LLVM `@__print_i64`, JVM `System.out`,
CLR `Console.WriteLine`, WASM `env.__print_i64`). The `port` argument is a
compile-time-constant hardware selector and, with all ports mapped to stdout, has no
effect, so it is not evaluated. `out` is statement-shaped (no value); the lowering
returns a fresh `const 0` for the discarded expression slot.

**Why this matters:** Oct had *no observable output* — its `main` is forced void
(always exits 0), so no Oct program could witness a computed result, which made the
LANG-MATRIX unable to verify any Oct value-level feature by running. With `out`, Oct
gets stdout, so its behaviour is now checkable: `lang-aot`'s `lang_matrix` battery
RUNS `out(1, 200)` → prints `200` and `out(1, 100 + 100)` → prints `200` (arithmetic
proven observably) across native / LLVM / WASM / JVM / CLR / VM / JIT. This unblocks
verification of the deferred Oct items (O1 short-circuit, O2, O3).

The other intrinsics (`in`, `adc`, `sbb`, `rlc`/`rrc`/`ral`/`rar`, `carry`, `parity`)
have no general-backend model yet and remain cleanly rejected. New unit tests:
`out_intrinsic_lowers_to_print_i64`, `out_of_computed_value`,
`other_intrinsics_still_rejected`.

## 0.4.0 — 2026-05-30 (OCT05 — source-location threading for debugger)

### Added — Real source positions in `IIRFunction.source_map`

Oct's emitted IIR now carries real `(line, column)` per instruction
in `IIRFunction.source_map`, in lockstep with `instructions`.
Previously the field was either empty or all `SourceLoc::SYNTHETIC`.

This is the prerequisite for line-based breakpoints in the future
`oct-dap` debugger crate.  Without real positions, the debug
sidecar built by the DAP layer cannot resolve `setBreakpoints
{ file, lines: [N] }` requests to IIR instructions.

### Implementation

- New `node_loc(&GrammarASTNode) -> SourceLoc` helper extracts
  `(start_line, start_column)` from an AST node, falling back to
  `SYNTHETIC` when the parser couldn't attach positions.
- `Compiler` gained two fields: `source_map: Vec<SourceLoc>` (the
  per-function accumulator) and `current_loc: Cell<SourceLoc>`
  (the "currently compiling" position).  Reset at the start of
  every `compile_fn` call.
- `Compiler::emit` now pushes `current_loc.get()` onto
  `source_map` for every instruction it appends, maintaining the
  lockstep invariant.
- `compile_stmt` calls `set_loc(node_loc(stmt))` on entry, so all
  instructions emitted while compiling a statement (including from
  sub-expressions) inherit the statement's source line.  This is
  the right granularity for line-based debuggers: per-statement,
  not per-expression-column.
- `compile_fn` reset/take semantics: each function gets its own
  source_map slice, moved onto `iir_fn.source_map` at the end.
  Defensive padding handles the rare case where pre-set_loc
  emission slipped through (dead today; cheap to keep).

### Tests

- 2 new unit tests:
  - `source_map_lockstep_with_instructions`: every function's
    `source_map.len() == instructions.len()`.
  - `source_map_carries_real_line_numbers`: a 3-line program
    produces tagged-line entries for both let statements (not
    just SYNTHETIC).
- All 11 existing lib tests still pass (13 total).
- 9 backend_compat + 4 jit_e2e tests still pass.
- Downstream `lang-aot` (8 + 17) continues to pass.

## 0.3.0 — 2026-05-29 (OCT04 — AOT backend acceptance proofs)

### Added — `tests/backend_compat.rs` exercises every IIR-to-* backend

Oct's emitted IIR is now proven by automated tests to be accepted by
the validators of every AOT backend (wasm, jvm, clr, beam).  This
closes the "Oct's IIR shape could regress without anyone noticing"
gap — the same shape Twig (`twig-ir-compiler/tests/backend_compat.rs`)
and Nib (`nib-iir-compiler/tests/backend_compat.rs`) already had.

### Coverage (9 tests)

| Group | Test | Asserts |
|---|---|---|
| Minimal | `oct_empty_main_accepted_by_every_backend` | `fn main() { }` |
| Minimal | `oct_return_constant_accepted_by_every_backend` | `fn answer() -> u8 { return 42; }` |
| Arithmetic | `oct_typed_add_accepted_by_every_backend` | `x + y` (u8) |
| Arithmetic | `oct_typed_sub_accepted_by_every_backend` | `x - y` (u8) |
| Comparison | `oct_typed_eq_accepted_by_every_backend` | `x == 5` |
| Comparison | `oct_typed_lt_accepted_by_every_backend` | `x < 10` |
| Control flow | `oct_if_else_accepted_by_every_backend` | `if … { … } else { … }` |
| Control flow | `oct_while_loop_accepted_by_every_backend` | `while n < 10 { n = n + 1 }` |
| Invariant | `oct_every_function_is_fully_typed` | every fn has `type_status == FullyTyped` |

All 9 pass on first run — proving Oct's IIR is shape-compatible with
every backend without further changes.  This is the AOT counterpart
to `tests/jit_e2e.rs` (which proves the JIT path).

### Dependencies

Added `iir-to-wasm`, `iir-to-jvm-class-file`, `iir-to-cil-bytecode`,
`iir-to-beam` as **dev-dependencies**.  None of them ship to runtime
consumers of `oct-iir-compiler`.

### Tests

- 9 backend_compat tests pass.
- 11 lib + 4 jit_e2e existing tests still pass.

## 0.2.0 — 2026-05-28 (OCT03 — JIT via GenericCirJit)

### Added — Oct programs JIT-compile via `jit-core::GenericCirJit`

With `jit-core::GenericCirJit` landed in `jit-core` 0.3.0, Oct gets a
real JIT **without a per-language Backend impl**.  Oct functions
compile through `JITCore::execute_with_jit` → `GenericCirJit` →
packed bytecode.

This is the second language (after Brainfuck and Dartmouth BASIC) to
plug into the LANG VM's JIT chain.  Unlike Brainfuck and BASIC,
which still ship their own per-language Backend impls
(`BrainfuckCirJit` / `BasicCirJit`), Oct uses `GenericCirJit`
directly — no duplicated code.

### Changed — `IIRFunction::type_status = FullyTyped` override

`IIRFunction::new`'s automatic `infer_type_status` returns
`PartiallyTyped` because Oct's control-flow ops (`label`, `jmp`,
`jmp_if_false`, `ret_void`) carry `"void"` hints, and `"void"` is
NOT in `interpreter_ir::opcodes::CONCRETE_TYPES`.  Every Oct
instruction is in fact statically known (no `"any"` hints), so the
function is genuinely fully typed for the JIT's threshold-zero
compile path.  We now override `type_status = FullyTyped` after
construction, mirroring Brainfuck and BASIC.

Without this fix, `JITCore` would never call `compile()` on Oct's
functions, and `GenericCirJit` would never run.

### Tests

- 4 new end-to-end tests in `tests/jit_e2e.rs`:
  - `oct_jit_returns_constant_42`: `fn answer() -> u8 { return 42; }`
  - `oct_jit_arithmetic_and_return`: `let x: u8 = 30; let y: u8 = 12;
    return x + y;` → 42
  - `oct_jit_if_else`: `if x == 0 { x = 1; } else { x = 2; }` → 1
  - `oct_jit_while_loop`: `while n < 10 { n = n + 1; }` → 10
- All 11 existing lib tests continue to pass.

### Dependencies

- Added `vm-core` and `jit-core` as **dev-dependencies** (the JIT
  test harness lives in `tests/jit_e2e.rs`).  Oct's main library
  has no runtime JIT dependency — the JIT integration is purely a
  consumer-side concern (downstream `oct-vm` or similar would pull
  in `vm-core` + `jit-core` as needed).

## 0.1.0 — 2026-05-20 (OCT02 phase 3)

Initial Rust port of the Oct IIR compiler.  Lowers a parsed +
type-checked Oct program to `interpreter_ir::IIRModule` ready for the
LANG VM AOT chain.

### What compiles (V1)

- Function declarations with parameters and return types.
- Cross-function calls + recursion (uses LANG43's cross-function reloc).
- Local variables (lowered to named IIR slots) and `mov` updates.
- Arithmetic `+` `-` → `add` / `sub`.
- Bitwise `&` `|` `^` → `and` / `or` / `xor`.
- Comparisons (`==` `!=` `<` `>` `<=` `>=`) → `cmp_*`.
- Logical `&&` `||` lowered as eager bitwise on 0/1 operands (the type
  checker already requires `bool` operands, so the truth values are
  preserved).
- Unary `!` / `~` → `not` (V1 doesn't distinguish bitwise NOT from
  logical NOT at the IIR level; on 0/1 operands the result is correct
  in both interpretations; full-width bitwise NOT for arbitrary `u8`
  is a V2 follow-up).
- `if`/`else`, `while`, `loop`, `break` via the canonical IIR loop
  scaffold (`label` / `jmp_if_false` / `jmp` / `label`).
- Integer / hex / binary literals → `const`.
- `true` / `false` → `const 1` / `const 0`.

### What's rejected

- Every 8008 hardware intrinsic (`in`, `out`, `adc`, `sbb`, `rlc`,
  `rrc`, `ral`, `rar`, `carry`, `parity`) → `OctError::Unsupported8008Intrinsic`.
- Type errors from the upstream type checker → `OctError::Type` with
  one message per diagnostic.
- Parser errors → `OctError::Parse`.

### Entry-point convention

The Oct language spec declares `fn main()` with void return.  The
LANG VM AOT chain expects `main` to return `i64` so the C runtime's
`exit()` truncation produces a sensible exit code.  This crate rewrites
Oct's void `main` to return `i64 0` so the chain works without any
backend changes.

### Tests

11 unit tests cover minimal main, arithmetic, if/else, while, loop +
break, cross-function calls, recursion, and every rejection path
(intrinsic, type error, parse error).

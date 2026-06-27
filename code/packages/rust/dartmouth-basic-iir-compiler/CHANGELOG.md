# Changelog — `dartmouth-basic-iir-compiler`

## 0.11.0 — 2026-06-27 — staged BA7 real arithmetic + whole-valued `PRINT`

BA7 starts the Dartmouth BASIC numeric cutover without forcing every existing
integer-oriented path to change in one blast:

- Decimal/exponent literals (`6.0`, `1E3`) now lower as `Operand::Float` with an
  `f64` type hint instead of being silently truncated to `i64`.
- Expression lowering carries a typed value (`i64` or `f64`) through arithmetic,
  `LET`, `IF`, `FOR`/`NEXT`, and `PRINT`. Mixed integer/real arithmetic widens
  the integer side with E8 `int_to_real` and emits the arithmetic op at `f64`.
  Integer-spelled programs still emit `i64`, so existing BA1/BA2/BA3/BA5/BA6
  cells keep their old slot shape.
- Scalar variables are promoted to real once a real value flows into them, so a
  backend never sees the same BASIC scalar slot flip between `i64` and `f64`.
  Integer-only boundaries remain explicit: array subscripts and integer array
  elements use E8 `real_to_int_trunc` when fed a real expression; `DATA` remains
  integer-only for now.
- `PRINT` chooses a new synthetic `__basic_print_real(x: f64)` helper for real
  items. This BA7-1 helper implements whole-valued output by truncating with E8
  `real_to_int_trunc` and delegating to the BA2 digit printer. Fractional
  formatting and `E` notation remain the next BA7 slice.
- Verified by frontend unit tests and an executed `lang-aot` matrix program:
  `PRINT 6.0 * 7.0` => `42` on native / LLVM / WASM / JVM / CLR / VM / JIT.

## 0.10.0 — 2026-06-26 — `GOSUB` / `RETURN` (LANG-FULL BA1, enabler E7)

`GOSUB` and `RETURN` were `UnsupportedStatement` rejections; they now lower onto
the **E5 array** substrate + an **AL5 computed-`goto`** — no new IIR op, exactly
as designed in `code/specs/lang-full-e7-subroutine-return-stack.md`.

BASIC's `GOSUB`/`RETURN` is *unstructured*: the program is one flat list of
line-numbered statements in `main`, and the same `RETURN` resumes at the
**dynamically most-recent** `GOSUB`. Plain `call`/`ret` can't express that, so we
model it *inside* `main`:

- A pre-pass counts every `GOSUB` (so a `RETURN` appearing before some of the
  `GOSUB`s it returns to still emits the full dispatch chain) and, when the
  program uses `GOSUB`, materialises a return-address stack at the top of `main`:
  a fixed-capacity (64) `array<i64>` (`__basic_gosub_stack`) plus the
  `__basic_gosub_sp` pointer — mirroring the BA6 `DATA`-pool init.
- **`GOSUB n`** pushes its 0-based call-site id (`array_set` + sp bump), `jmp`s to
  `line_n`, and drops a `gosub_ret_<id>` resume label.
- **`RETURN`** pops the id (`array_get` after sp decrement) and computed-`goto`s
  to its `gosub_ret_<id>` via the AL5 switch chain (`cmp_eq` + `jmp_if_true` over
  every site). A bare `RETURN` (no `GOSUB` in the program) is a clean error;
  over-deep nesting traps via the bounds-checked `array_set` (the faithful
  "GOSUB too deep" runtime error).
- Every op (`alloc_array`/`array_set`/`array_get`, `const`/`add`/`sub`/`mov`,
  `cmp_eq`/`jmp`/`jmp_if_true`/`label`) already runs on every backend, so **BA1
  added ZERO backend ops** — proven by two executed `lang-aot` matrix programs:
  `GOSUB 100` twice with one shared `RETURN` ⇒ `919` (same `RETURN`, two sites),
  and a nested `GOSUB` ⇒ `876` (LIFO across depth > 1). Six new frontend unit
  tests.
- *Known gap (BA1-WASM):* the `RETURN` computed-`goto` produces an irreducible
  CFG that trips `iir-to-wasm`'s dispatch-loop lowering with a runtime
  `StackUnderflow` (the wasm *compiles* but traps), so the GOSUB matrix cells run
  on the other **six** backends (native/LLVM/JVM/CLR/VM/JIT) pending a focused
  iir-to-wasm fix.

## 0.9.0 — 2026-06-26 — multi-item `PRINT` on one line (LANG-FULL BA2)

`PRINT` could only print a single value per statement, because each item lowered
to `call_builtin "print_i64"` and `print_i64` appends a newline — so `PRINT A; B`
wrongly split `A` and `B` onto separate lines. BA2 moves BASIC to a
**character-level output model** that lets several items share a line:

- **Two synthetic helper functions** are appended to the module (only when a
  program actually prints a value): `__basic_print_uint(n)` renders an
  unsigned magnitude by **recursing** on `n / 10` (high-order digits first, so
  digits come out left-to-right with no reversal buffer) and emitting each digit
  with the universal `putchar` builtin; `__basic_print_int(n)` handles the sign
  (`putchar('-')` then the magnitude) and dispatches to it. Both use **only**
  ops every backend already runs — `const`, `cmp_*`, `div`/`mul`/`sub`/`add`,
  `call` (the ALGOL value-procedure ABI), `jmp`/`label`, and `putchar` (shared
  with Brainfuck) — so BA2 required **zero backend changes** and runs on all
  seven targets (NativeAot/LLVM/WASM/JVM/CLR/VM/JIT).
- **`emit_print`** now walks the `print_list` in source order, lowering each
  numeric item to a `call __basic_print_int`, applying separator spacing, and
  emitting a trailing `putchar(10)` newline unless the list ends on a separator.
- **Separators:** `;` joins items tightly (nothing between); `,` inserts a
  single space. A **trailing** separator (`PRINT X;` / `PRINT X,`) suppresses
  the final newline. Bare `PRINT` now emits a blank line (a lone newline)
  instead of being a no-op.
  - *Divergence (documented):* historical Dartmouth BASIC tabs `,` to the next
    14-column **print zone**; that needs a run-time output-column counter and is
    deferred to a later item. A single space is BA2's well-defined approximation.
  - *Limitation:* the sign path negates with `0 - n`, which overflows only at
    `i64::MIN` — a value no BA2 program can express; a saturating negate is a
    later refinement.
- **String `PRINT` items** still error (`need LANG77`) — they wait for BA4 /
  enabler E4. **More relops** were already in place (the grammar and
  `extract_relop_op` cover all six: `= < > <= >= <>`); BA2 adds no relop work.
- Proven by two executed `lang-aot` matrix cells (`PRINT 0 - 12; 34` ⇒ `-1234`,
  `PRINT 5, 6` ⇒ `5 6`) that run on all 7 backends, plus six new frontend unit
  tests. The existing single-item BASIC matrix cells now route through `putchar`
  too and still pass unchanged.

## 0.8.0 — 2026-06-22 — `READ` / `DATA` / `RESTORE` (LANG-FULL BA6)

`READ`, `DATA`, and `RESTORE` were `UnsupportedStatement` rejections; they now
lower onto the **E5 array** substrate (no new IIR op, no enabler needed):

- A pre-pass gathers every `DATA` numeric literal across the whole program in
  line-number order into a pool. The pool is materialised **once at the top of
  `main`** as an `array<i64>` (`alloc_array` + one `array_set` per value) plus
  an `__basic_data_ptr` register seeded to 0. Because the program is a single
  `main` function (no `GOSUB`), the pointer lives in a register and persists
  across `READ`s — no module global needed.
- `READ var {, var}` fetches `array_get pool, __basic_data_ptr` into each target
  (a scalar `mov`, or an `array_set` for `READ A(I)`) and advances the pointer
  (`__basic_data_ptr := __basic_data_ptr + 1`). Reading past the pool traps via
  the bounds-checked `array_get` — the "out of DATA" runtime error.
- `RESTORE` rewinds the pointer to 0; `DATA` itself emits nothing at its line.
- **Limitations:** integer `DATA` only (a real/`f64` value is a clean
  `Unsupported` error — a follow-up, like integer-only `DIM` arrays); `READ`/
  `RESTORE` with no `DATA` in the program is a clean error.

Verified by **running**: a `lang_matrix.rs` cell (`DATA 21 / READ A / RESTORE /
READ B / PRINT A+B` ⇒ `42`, proving sequential consumption *and* the rewind)
runs on **all 7 backends**; plus JIT-run tests (`READ X` ⇒ 42; `READ A,B` +
`RESTORE` + `READ C` ⇒ 10,20,10) and four structural unit tests.

## 0.7.0 — 2026-06-21 — `DIM` arrays (LANG-FULL BA3, enabler E5)

### Added — one-dimensional integer arrays (`DIM A(n)` + subscripted `A(i)`)

`DIM` was previously an `UnsupportedStatement` and a subscripted variable `A(I)`
was a deferred `Unsupported` error. BASIC arrays now lower to the shared IIR
array ops (the *same* `alloc_array` / `array_set` / `array_get` ALGOL's E5 arrays
use), so they run on every backend E5 already supports:

```basic
10 DIM A(3)
20 LET A(1) = 40
30 LET A(2) = 2
40 PRINT A(1) + A(2)
50 END
```

⇒ `alloc_array A = 4` (see below) then `array_set A, 1, 40` / `array_set A, 2, 2`
and two `array_get`s feeding the `add`, printing `42`.

- **`DIM A(n)` → `alloc_array`** — Dartmouth BASIC arrays are **0-based and
  inclusive**: `DIM A(3)` declares `A(0)..A(3)`, so the element count is
  `n + 1` (here `4`). `DIM A(3), B(2)` declares both in one statement. The
  handle lives in the register named after the array.
- **`LET A(i) = e` → `array_set`** and **`A(i)` in an expression → `array_get`**.
  The subscript is used **directly** as the 0-based IIR index — no lower-bound
  subtraction (contrast ALGOL's `array A[lo:hi]`, which subtracts `lo`).
- **Undeclared use is a clean error**: subscripting a name that was never
  `DIM`med returns `Unsupported` rather than miscompiling against an undefined
  handle register.
- **Bounded + panic-free**: a `DIM` bound is validated against `MAX_DIM_BOUND`
  (2^24) *before* the `f64`→`i64` cast, so an oversized or non-finite literal
  (e.g. `DIM A(1E30)`, which a bare `as i64` cast would saturate to `i64::MAX`
  and then overflow on the `+ 1` element count) is a clean `Unsupported` error,
  never a debug-build panic or a release-build wrapped/garbage length. The
  element-count `+ 1` also uses `checked_add` as a second guard.
- **Verified by RUNNING** the straight-line array program across the matrix
  (`lang-aot` `tests/lang_matrix.rs`): the managed runtimes bounds-check natively
  (JVM `int[]`/`iastore`/`iaload`, CLR `int32[]`/`stelem`/`ldelem`), the static
  backends use the length-prefixed block + explicit bounds-trap (LLVM/WASM/
  NativeAot). 7 new unit tests cover the lowering shape + the error paths.

## 0.6.0 — 2026-06-20 — `DEF FN` user-defined functions (LANG-FULL BA5)

### Added — single-line user-defined functions (`DEF FNx(P) = expr`)

`DEF` was previously an `UnsupportedStatement`. A single-line BASIC function
now lowers to a **sibling `IIRFunction`** and `FNx(arg)` call sites lower to the
shared IIR `call` op — the same calling convention ALGOL's value procedures
(AL3) already run on every backend:

```basic
10 DEF FNS(X) = X * X
20 PRINT FNS(7)
30 END
```

⇒ `fn FNS(X: i64) -> i64 { ret X * X }` plus `call FNS, 7` in `main`, printing
`49`. **Verified by RUNNING** across native / LLVM / WASM / JVM / CLR / VM / JIT
(`lang-aot` `tests/lang_matrix.rs`).

Mechanics, mirroring `algol-iir-compiler`'s `compile_procedure`:

- A **pre-pass** registers every `DEF FNx` name before any statement is lowered,
  so a program may *call a function on an earlier line than its `DEF`* (BASIC
  permits forward use).
- Each `DEF` body is lowered in a swapped-in emission context (its own
  instruction stream / temp counter / source map), then assembled into a
  `FullyTyped` `IIRFunction` pushed onto the module **after** `main`.
- A `FNx(arg)` call evaluates its single argument and emits
  `call dest = [callee, arg]` with an `i64` return hint.

**Limits (follow-ups):** one numeric parameter only (per the 1964 grammar); the
body may reference **only its parameter** — global access from inside a function
needs the host global table the code-gen backends reject (enabler **E6**), so any
other variable reference is a clean `Unsupported` error rather than an
undefined-register miscompile. Built-in maths functions (`SIN`/`ABS`/…) stay
deferred until E3 (reals).

A companion fix in `lang-aot` 0.94.0 made the JVM scalar-concretization
**module-consistent** so the printing `main` and its non-printing callee `FNS`
share one value model (see that crate's changelog).

## 0.5.0 — 2026-06-13 — comparisons emit the operand width; control flow runs on the code-gen backends (LANG-FULL BA0)

### Fixed — `IF`/`FOR` comparisons emitted a `bool` type hint, breaking LLVM (and WASM)

`IF e1 relop e2` and `FOR`/`NEXT` emitted their `cmp_*` with `type_hint = "bool"`
(the result type). But a comparison's IIR `type_hint` is the **operand** width,
which the IIR-to-* backends use to size the machine compare: LLVM emitted
`icmp <op> i1` — a 1-bit compare that truncates the i64 operands (`7 > 5` became
`1 > 1` → false), so `IF A > 5` fell through and `FOR` mis-looped. The compiler now
emits the operand type `i64` (matching Nib / Oct / ALGOL); the boolean *result* is
implicit, exactly as those languages already do.

This is why BASIC control flow previously ran only on the VM/JIT. With the fix,
`lang-aot`'s `lang_matrix` battery RUNS a `FOR`/`NEXT` accumulator loop
(`FOR I = 1 TO 5: S = S + I` → prints 15) and an `IF A > 5 THEN 100` jump
(prints 7) across native / LLVM / WASM / CLR / VM / JIT.

### Removed — two stale `#[ignore]`s in `tests/backend_encode.rs`

`basic_control_flow_lowers_to_wasm_bytes` and `basic_for_loop_lowers_to_wasm_bytes`
were ignored on the premise that `iir-to-wasm` couldn't lower `cmp_gt`/`cmp_le`;
that gap has since been closed (the wasm lowering grew the full `cmp_*` table), so
both tests pass and are re-enabled.

### Known follow-up — BA-JVM-1

BASIC programs combining a **branch** (`IF`/`FOR`) with a `print_i64` call do not yet
run on the JVM (output is empty) — the `iir-to-jvm-class-file` StackMapTable
generation trips on the frame at the branch target when several `long` locals are
live across a host-method invoke. (A print with no branch — `10 PRINT 42` — and a
loop with no print — Nib's for-loops — both run on JVM; only the combination fails.)
The JVM cell is excluded for the two control-flow matrix programs pending that fix.

## 0.4.0 — 2026-05-30 (BASIC05 — source-location threading for debugger)

### Added — Real source positions in `IIRFunction.source_map`

BASIC's emitted IIR now carries real `(line, column)` per instruction
in `IIRFunction.source_map`, in lockstep with `instructions`.
Previously the field was either empty or all `SourceLoc::SYNTHETIC`.

This is the prerequisite for line-based breakpoints in the future
`basic-dap` debugger crate.  Without real positions, the debug
sidecar built by the DAP layer cannot resolve `setBreakpoints
{ file, lines: [N] }` requests to IIR instructions.

This mirrors the pattern landed for `oct-iir-compiler` 0.4.0
(OCT05 / PR #4583).  Same `node_loc()` + `Cell<SourceLoc>` +
statement-level `set_loc()` shape — the next step in the
horizontally-sequenced "every language gets every Twig-grade
tool" roadmap.

### Implementation

- New `node_loc(&GrammarASTNode) -> SourceLoc` helper extracts
  `(start_line, start_column)` from an AST node, falling back to
  `SYNTHETIC` when the parser couldn't attach positions.
- `Compiler` gained two fields: `source_map: Vec<SourceLoc>` (the
  per-function accumulator) and `current_loc: Cell<SourceLoc>`
  (the "currently compiling" position).  Manual `impl Default`
  replaces the `#[derive(Default)]` since `Cell<SourceLoc>` doesn't
  have a usable default (well, it does — but being explicit makes
  the SYNTHETIC start state obvious to readers).
- `Compiler::emit` now pushes `current_loc.get()` onto `source_map`
  for every instruction it appends, maintaining the lockstep
  invariant.
- `emit_line` calls `set_loc(node_loc(line))` on entry — all
  instructions emitted for that line (label + body) inherit the
  line's source position.
- `emit_statement` re-tags with the wrapped statement node's own
  position, which may be a tighter range than `emit_line` set.
- `emit_program` sets the initial loc to the program root so the
  synthesised end-of-program epilogue (`const 0; ret`) gets a
  sensible source line rather than `SYNTHETIC`.
- `compile_program` ends with the move-with-defensive-padding shape:
  `main.source_map = std::mem::take(&mut comp.source_map)` after
  ensuring `source_map.len() == instructions.len()`.

### Tests

- 2 new unit tests:
  - `source_map_lockstep_with_instructions`: every function's
    `source_map.len() == instructions.len()`.
  - `source_map_carries_real_line_numbers`: a 4-line BASIC program
    produces entries for every line — proving the per-line source
    positions get threaded through, not just SYNTHETIC.
- All existing lib tests still pass.

## 0.3.0 — 2026-05-29 (PL05-C — AOT backend acceptance proofs)

### Added — `tests/backend_compat.rs` exercises every IIR-to-* backend

BASIC's emitted IIR is now proven by automated tests to be accepted
by the validators of every AOT backend (wasm, jvm, clr, beam).  This
closes the "BASIC's IIR shape could regress without anyone noticing"
gap — the same shape Twig (`twig-ir-compiler/tests/backend_compat.rs`),
Nib (`nib-iir-compiler/tests/backend_compat.rs`), and Oct (PR #4580)
already had.

### Coverage (8 tests)

| Group | Test | Asserts |
|---|---|---|
| Minimal | `basic_minimal_end_accepted_by_every_backend` | `10 END` |
| Minimal | `basic_let_binding_accepted_by_every_backend` | `LET A = 42` |
| Arithmetic | `basic_typed_add_accepted_by_every_backend` | `C = A + B` |
| Arithmetic | `basic_typed_mul_accepted_by_every_backend` | `C = A * B` |
| Control flow | `basic_if_then_goto_accepted_by_every_backend` | `IF A > 5 THEN 100` |
| Control flow | `basic_for_next_loop_accepted_by_every_backend` | `FOR I = 1 TO 3 / NEXT I` |
| Control flow | `basic_goto_accepted_by_every_backend` | `GOTO 100` |
| Invariant | `basic_main_is_fully_typed` | main has `type_status == FullyTyped` |

All 8 pass on first run — BASIC's IIR is shape-compatible with every
backend with zero further changes.  This is the AOT counterpart to
the existing tests/jit_smoke.rs + tests/jit_real_backend.rs (which
prove the JIT path).

### Dependencies

Added `iir-to-wasm`, `iir-to-jvm-class-file`, `iir-to-cil-bytecode`,
`iir-to-beam` as **dev-dependencies**.  None of them ship to runtime
consumers of `dartmouth-basic-iir-compiler`.

### Tests

- 8 new backend_compat tests pass.
- 17 lib + 8 + 6 + 4 existing tests still pass.

## 0.2.0 — 2026-05-26 (PL05-B — real BasicCirJit backend)

### Added — `BasicCirJit`: a real `jit_core::backend::Backend`

Ships a real bytecode JIT for Dartmouth BASIC, modelled on Brainfuck's
`BrainfuckCirJit` pattern.  Translates the specialised CIR instruction
stream (`const_i64`, `add_i64`, `cmp_*_i64`, `jmp`, `jmp_if_false`,
`call_builtin "print_i64"`, `ret_void`) into a packed register-machine
bytecode and interprets it in a tight match-loop — bypassing
`vm-core`'s generic IIR dispatch entirely.

Same "classic JIT" shape used by the JVM Ignition tier, Smalltalk-80,
Lua, and V8 Ignition.  Not a native-code JIT (Cranelift / x86_64) —
swapping in a native backend later is the only change needed.

#### Bytecode opcodes

22 opcodes covering BASIC's full V1 vocabulary:
- Constants: `CONST_I64` (8-byte little-endian payload)
- Arithmetic: `ADD_I64` / `SUB_I64` / `MUL_I64` / `DIV_I64` / `NEG_I64`
- Comparisons: `CMP_EQ_I64` / `CMP_NE_I64` / `CMP_LT_I64` / `CMP_LE_I64`
  / `CMP_GT_I64` / `CMP_GE_I64`
- Control flow: `JMP` / `JMP_IF_FALSE` / `JMP_IF_TRUE` (i16 LE offsets)
- Builtins: `PRINT_I64` / `INPUT_I64`
- Returns: `RET_I64` / `RET_VOID`
- Plus `MOV` for register-to-register copy

Register file: 256 i64 registers, single-byte indices.

#### Shared I/O via Arc<Mutex<…>>

`BasicCirJit::new` takes `Arc<Mutex<Vec<i64>>>` (output),
`Arc<Mutex<VecDeque<i64>>>` (input), `Arc<Mutex<u64>>` (step counter),
and `Arc<Mutex<Option<String>>>` (error slot).  The same Arc handles
can be shared with `VMCore`'s `print_i64` / `input_i64` builtin
registrations, so interpreter-fallback and JIT-compiled paths see the
same logical streams.

#### `main.type_status = FullyTyped` override

BASIC's IIR uses `"void"` type hints on control-flow ops (`label`,
`jmp`, `ret`, `call_builtin "print_i64"`).  `"void"` is **not** in
`interpreter_ir::opcodes::CONCRETE_TYPES`, so `IIRFunction::new`'s
automatic `infer_type_status` returns `PartiallyTyped`.  Without an
explicit override, `jit-core`'s threshold-zero compile path (which
requires `FullyTyped`) would never fire.

The fix mirrors Brainfuck's compiler: after `IIRFunction::new`, set
`main.type_status = FunctionTypeStatus::FullyTyped`.  Every BASIC
instruction is in fact statically known (no `"any"` hints anywhere),
so the override is semantically correct.

#### Tests

- 5 unit tests in `jit_backend::tests` cover compile + run paths for
  CONST_I64 / RET_I64, PRINT_I64, ADD_I64, unknown-opcode rejection,
  and division-by-zero error reporting.
- 6 end-to-end integration tests in `tests/jit_real_backend.rs` run
  full BASIC programs through `JITCore` with `BasicCirJit` as the
  backend (instead of `NullBackend`).  Covers PRINT, LET +
  arithmetic, FOR loops, IF/GOTO branches, multiplication, and
  accumulating FOR with arithmetic in the body.

### Changed

- `jit-core` and `vm-core` promoted from dev-dependencies to main
  dependencies — `BasicCirJit` lives in this crate's `src/`, not its
  `tests/`.
- `jit_backend` module re-exported from `lib.rs`; `BasicCirJit`,
  `DEFAULT_OUTPUT_CAP`, and `DEFAULT_STEP_CAP` are part of the public
  API.

## 0.1.0 — 2026-05-20 (PL05 initial release)

Initial release.  Compiles Dartmouth BASIC source to
`interpreter_ir::IIRModule`, unlocking the LANG VM AOT chain
(twig-aot / lang-aot → x86_64-backend / aarch64-backend → object →
system linker → native executable) for BASIC programs.

Distinct from the existing `dartmouth-basic-ir-compiler` crate, which
targets the GE-225 simulator's custom `compiler_ir::IrProgram` shape
and is not pluggable into the LANG VM chain.

### V1 coverage (integer programs)

| Statement | Status |
|-----------|--------|
| `LET A = expr` | ✓ |
| `PRINT expr`   | ✓ (numeric only — strings deferred to LANG77) |
| `INPUT X`      | ✓ |
| `IF cond THEN m` | ✓ |
| `GOTO m`       | ✓ |
| `FOR I = a TO b STEP s` / `NEXT I` | ✓ (positive STEP) |
| `END` / `STOP` | ✓ |
| `REM …`        | ✓ (no-op) |
| `GOSUB` / `RETURN` | **deferred** — V1 errors with `UnsupportedStatement` |
| `READ` / `DATA` / `RESTORE` | deferred — needs data pool |
| `DIM` / arrays | deferred — needs LANG76-based byte arrays |
| `DEF`          | deferred |

### Expression coverage

- Integer literals (floats truncate to i64; explicit float support
  deferred until backends grow SSE2).
- Variables (scalar `A..Z`, `A0..Z9` — array access `A(I)` deferred).
- Arithmetic: `+`, `-`, `*`, `/` with standard precedence.
- Unary minus.
- Exponentiation (`^`): deferred — needs a runtime helper.
- Built-in / user-defined functions (`SIN`, `FNA`, …): deferred.

### IIR shape

The whole program becomes a single function `main` returning `i64`.
Every BASIC line gets a label `line_<n>`; flow-control statements
jump between those labels.  FOR/NEXT loops use per-loop synthetic
labels `for_<id>_test` / `for_<id>_end`.

### Tests

11 unit tests cover each supported statement plus the deferred
`UnsupportedStatement` paths.  End-to-end smoke tests in
`lang-aot/tests/end_to_end_smoke.rs` compile BASIC programs all the
way to native executables on Windows + Linux and assert stdout:

- `10 PRINT 42 / 20 END` → stdout `"42\n"`.
- `10 FOR I = 1 TO 3 / 20 PRINT I / 30 NEXT I / 40 END` → stdout
  `"1\n2\n3\n"`.

Spec: `code/specs/PL05-dartmouth-basic-iir-compiler.md`.

# Changelog — `dartmouth-basic-iir-compiler`

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

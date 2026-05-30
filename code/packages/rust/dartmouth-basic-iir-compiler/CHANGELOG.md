# Changelog — `dartmouth-basic-iir-compiler`

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

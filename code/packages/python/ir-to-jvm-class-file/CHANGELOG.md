# ir-to-jvm-class-file

## 0.6.1 — 2026-04-28

### Fixed — JVM01: caller-saves around `IrOp.CALL` so recursion works on real `java`

The "register" model uses a class-level static `int[]` array
shared across every `invokestatic`, so a recursive call would
clobber the caller's register values.  `IrOp.CALL` emission now
snapshots every register slot into JVM locals immediately before
the `invokestatic` and restores them immediately after (skipping
`r1`, the return-value slot).  Each callable's `max_locals` is
bumped to `reg_count` to cover the snapshot stash.

This is the minimal-diff path described in
`code/specs/JVM01-jvm-per-method-locals.md` — the bigger
descriptor rewrite stays as a future cleanup option.

Regression test:
`tests/test_oct_8bit_e2e.py::test_call_preserves_caller_registers`.

## 0.6.0 — 2026-04-27

### Added — LANG20: `JVMCodeGenerator` — `CodeGenerator[IrProgram, JVMClassArtifact]` adapter

**New module: `ir_to_jvm_class_file.generator`**

- `JVMCodeGenerator` — thin adapter satisfying the
  `CodeGenerator[IrProgram, JVMClassArtifact]` structural protocol (LANG20).

  - `name = "jvm"` — unique backend identifier.
  - `validate(ir) -> list[str]` — delegates to `validate_for_jvm()`.  Never
    raises; returns `[]` for valid programs.
  - `generate(ir) -> JVMClassArtifact` — delegates to
    `lower_ir_to_jvm_class_file(ir, config)`.  Raises on invalid IR.
  - Optional `config: JvmBackendConfig` — forwarded to the underlying compiler.

- `JVMCodeGenerator` exported from `ir_to_jvm_class_file.__init__`.

**New tests: `tests/test_codegen_generator.py`** — 14 tests covering: `name`,
`isinstance(gen, CodeGenerator)` structural check, `validate()` on valid / bad-
SYSCALL / overflow-constant IR, `generate()` returns `JVMClassArtifact`,
`class_bytes` starts with JVM magic `0xCAFEBABE`, `class_bytes` non-empty,
`generate()` raises on invalid IR, custom config accepted, round-trip, export
check.

---

## [Unreleased]

### Added

- **Oct 8-bit arithmetic e2e tests** (`tests/test_oct_8bit_e2e.py`):
  7 end-to-end tests confirming the JVM backend correctly compiles and
  executes 8-bit integer arithmetic IR — the same IR that the Oct compiler
  generates.  Tests cover: LOAD_IMM, ADD, SUB, AND (inc. 0xFF masking),
  multi-output programs, and validation of Oct's unsupported SYSCALL numbers.
  Execution uses the system ``java`` binary; tests are skipped if ``java``
  is not on PATH.  Key findings:
  - Pure 8-bit arithmetic compiles to standard JVM .class files and runs
    correctly through the full IR → JVM → java subprocess pipeline.
  - Oct's I/O intrinsics (SYSCALL 40+PORT / 20+PORT) are correctly rejected
    by the JVM validator.  The JVM backend only supports SYSCALL 1 and 4.

## 0.5.0 — 2026-04-20

### Added

- **`IrOp.OR` support**: emits `ior` (`0x80`) for register-register bitwise OR.
- **`IrOp.OR_IMM` support**: emits `ior` (`0x80`) for register-immediate bitwise OR.
- **`IrOp.XOR` support**: emits `ixor` (`0x82`) for register-register bitwise XOR.
  New opcode constant `_OP_IXOR = 0x82` added alongside the existing `_OP_IOR`.
- **`IrOp.XOR_IMM` support**: emits `ixor` (`0x82`) for register-immediate bitwise XOR.
- **`IrOp.NOT` support**: emits the source register value, then `iconst_m1` (`0x02`)
  to push -1 (all 32 bits set), then `ixor` (`0x82`) to flip every bit.  This
  correctly implements two's-complement bitwise NOT: `NOT(x) = x XOR 0xFFFFFFFF`.
- All five new opcodes added to `_JVM_SUPPORTED_OPCODES` so the pre-flight
  validator accepts them without error.
- 15 new tests in `TestValidateForJvm` covering:
  - Validator acceptance of OR, XOR, NOT, OR_IMM, XOR_IMM.
  - Successful lowering to structurally-valid class files for all five ops.
  - Bytecode-presence checks confirming `ior` (0x80) and `ixor` (0x82) appear
    in generated output, and that NOT specifically emits `iconst_m1` + `ixor`.
- Removed the `_BITWISE_V1_UNSUPPORTED` frozenset and the
  `test_bitwise_opcodes_are_intentionally_unsupported` test now that all five
  ops are implemented.  `test_all_supported_opcodes_pass_opcode_check` now
  iterates every `IrOp` without exclusions.

### Motivation

These opcodes were blocking end-to-end Oct → JVM compilation: Oct programs use
the `|`, `^`, and `~` operators which lower to `OR`, `XOR`, and `NOT` in
`compiler_ir`.  All three map directly to single JVM instructions.

## 0.4.0 — 2026-04-20

### Added

- **`validate_for_jvm(program)` pre-flight validator**: inspects an
  `IrProgram` for JVM backend incompatibilities *before* any bytecode is
  generated.  Returns a list of human-readable error strings (empty list =
  valid).  Three rules are checked:
  1. **Opcode support** — every opcode must appear in the V1 supported set.
     Currently all `IrOp` values are handled; the check is future-proofing
     against new IR opcodes added before the JVM backend implements them.
  2. **Constant range** — `LOAD_IMM` and `ADD_IMM` immediates must fit in a
     JVM 32-bit signed integer (−2 147 483 648 to 2 147 483 647).
  3. **SYSCALL number** — only SYSCALL 1 (write byte) and SYSCALL 4 (read
     byte) are wired up in the V1 JVM backend.
- `validate_for_jvm` exported from `ir_to_jvm_class_file.__init__`.
- `TestValidateForJvm` test class (14 tests) covering all three rules,
  boundary-value constants, multi-error accumulation, and integration with
  `lower_ir_to_jvm_class_file`.

### Changed

- `lower_ir_to_jvm_class_file()` now calls `validate_for_jvm()` as a
  pre-flight check before `_JvmClassLowerer` runs.  Any violation raises
  `JvmBackendError` with message prefix
  `"IR program failed JVM pre-flight validation"`.

## 0.3.0 — 2026-04-20

### Changed

- **`JvmBackendConfig.syscall_arg_reg` field removed.**  The SYSCALL IR
  instruction now carries the arg register as `operands[1]` (an `IrRegister`),
  so the backend reads the register index directly from the instruction rather
  than from a config parameter.  Callers no longer need to pass
  `syscall_arg_reg=0` for BASIC or `syscall_arg_reg=4` for Brainfuck.

- **`__ca_syscall` helper descriptor changed from `(I)V` to `(II)V`.**
  The helper method now accepts two `int` parameters: syscall number and
  arg-register index.  The WRITE path loads `__ca_regs[arg_reg]` at runtime
  using the passed-in register index instead of a compile-time constant.
  The READ path stores the byte received from stdin in local 2 (shifted up from
  local 1 to make room for the new arg-register parameter in local 1).
  Max locals increased from 2 to 3 accordingly.

## 0.2.0 — 2026-04-19

### Added

- `IrOp.MUL` support: emits `imul` (`0x68`) so Dartmouth BASIC multiplication
  expressions (`LET A = B * C`, `PRINT 3 * I`) lower correctly to JVM bytecode.
- `IrOp.DIV` support: emits `idiv` (`0x6C`) so Dartmouth BASIC integer division
  expressions lower correctly.  Integer division truncates toward zero, matching
  Dartmouth BASIC semantics.

## 0.1.0 - 2026-04-17

- Add the initial Python prototype for lowering `compiler_ir.IrProgram` to JVM
  class-file bytes.
- Emit a single generated class with static register and memory fields.
- Add helper-method-based lowering for byte memory, word memory, syscalls,
  branching, arithmetic, and comparisons.
- Add frontend integration tests for Brainfuck and Nib IR producers.
- Add GraalVM runtime smoke tests that execute generated Brainfuck and Nib
  programs on a locally installed GraalVM JDK and compile them with
  `native-image`.
- Harden class-name validation and class-file writes so malformed names cannot
  escape the requested output directory.
- Flush stdout in the generated write syscall helper so captured JVM/native
  output is observable in end-to-end tests.
- Fix the package `BUILD` file to install Brainfuck's transitive
  `virtual-machine` dependency during local test setup.
- Declare the package's test-only sibling dependencies in `pyproject.toml`
  so the build validator accepts the BUILD graph during CI, and remove the
  now-redundant standalone `grammar-tools` editable install from `BUILD`.
- Rewrite `write_class_file()` to anchor output writes on directory file
  descriptors and reject symlinked path components, closing a symlink-race
  overwrite hole in the original output-path validation.
- Bound total static data size and switch non-zero data initialization to
  compact `java.util.Arrays.fill()` range calls so hostile IR cannot explode
  the generated class initializer into a denial-of-service sized method body.

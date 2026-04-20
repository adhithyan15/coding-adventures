# ir-to-jvm-class-file

## 0.2.0 — 2026-04-19

### Added

- `IrOp.MUL` support: emits `imul` (`0x68`) so Dartmouth BASIC multiplication
  expressions (`LET A = B * C`, `PRINT 3 * I`) lower correctly to JVM bytecode.
- `IrOp.DIV` support: emits `idiv` (`0x6C`) so Dartmouth BASIC integer division
  expressions lower correctly.  Integer division truncates toward zero, matching
  Dartmouth BASIC semantics.
- `syscall_arg_reg` field on `JvmBackendConfig` (default `4`) — selects which
  IR virtual register holds the SYSCALL print argument.  The default keeps full
  backwards compatibility with Brainfuck IR (register 4).  Pass
  `syscall_arg_reg=0` when lowering Dartmouth BASIC IR, which places the
  print argument in register 0.

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

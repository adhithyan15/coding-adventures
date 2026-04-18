# ir-to-jvm-class-file

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

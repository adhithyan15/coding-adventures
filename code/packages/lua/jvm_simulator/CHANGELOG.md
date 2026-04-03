# Changelog — coding-adventures-jvm-simulator

## 0.1.0 — 2026-03-31

### Added

- Initial Lua port of the JVM simulator (Elixir reference: `elixir/jvm_simulator`).
- Full educational simulator for a subset of JVM integer bytecode.
- Opcodes: iconst_0-5, bipush, sipush, ldc (constant pool), iload_0-3, iload (with operand), istore_0-3, istore (with operand), iadd, isub, imul, idiv, goto, if_icmpeq, if_icmpgt, ireturn, return.
- Assembly helpers: `encode_iconst`, `encode_istore`, `encode_iload`, `assemble`.
- to_i32() wrapping for int32 overflow simulation.
- Step tracing: every instruction produces a trace with pc, opcode, stack_before, stack_after, locals snapshot, and description.
- `run()` function with configurable max_steps.
- 95%+ test coverage with busted.

# Changelog — CodingAdventures::JvmSimulator

## 0.01 — 2026-03-31

### Added

- Initial Perl port of the JVM simulator (Elixir reference: `elixir/jvm_simulator`).
- All integer opcodes: iconst_0-5, bipush, sipush, ldc, iload/istore (0-3 and long forms), iadd, isub, imul, idiv, if_icmpeq, if_icmpgt, goto, ireturn, return.
- Assembly helpers: encode_iconst, encode_istore, encode_iload, assemble.
- Step tracing with pc, opcode, stack_before/after, locals, description.
- run() with max_steps support.
- int32 overflow wrapping via _to_i32().
- JVM branch offset convention: target = instruction_pc + offset (not next_pc).
- 95%+ test coverage with Test2::V0.

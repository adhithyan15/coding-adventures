# Changelog

All notable changes to the jvm-simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] — 2026-04-20

### Added

- `static_fields: dict[object, object]` attribute on `JVMSimulator`, accepted as an
  optional constructor parameter.  When the same dict instance is passed to multiple
  simulators (or across sequential `load_method` calls) the stored field values persist
  across invocations — enabling the JVM dispatch-loop runtime to maintain shared
  register and variable state without a separate context object.
- `putstatic` — pops one value and stores it under the constant-pool field reference
  in `static_fields`.
- `getstatic` — now checks `static_fields` first (for user-defined program state such
  as register arrays) before falling back to the host's `get_static` method (for JVM
  stdlib references like `System.out`).  Existing tests that rely on the host path are
  unaffected.
- `invokestatic` — dispatches to `host.invoke_static(reference, static_fields, args)`.
  The descriptor is parsed to determine argument count and whether a return value
  should be pushed.  The host receives `static_fields` so it can read or mutate
  program-level state.
- Array opcodes: `newarray` (allocates a Python list of zeros), `iaload` / `iastore`
  (32-bit integer array load/store with `_to_i32` normalization on store), `baload`
  (byte load with sign-extension to int), `bastore` (byte store truncating to 8 bits).
- Single-operand branch opcodes `ifeq` and `ifne` via the new `_do_if_zero` helper,
  which pops one integer and branches if the condition against zero holds.
- Two-operand branch opcodes `if_icmplt` (less-than) and `if_icmpne` (not-equal) via
  the existing `_do_if_icmp` dispatch pattern.
- `iconst_m1` — added to the literal-push dispatch set alongside `iconst_0..5`,
  `bipush`, and `sipush`.
- `ldc_w` — wide form of `ldc`; decodes and pushes a constant-pool value identically
  to `ldc` but with a two-byte index operand.
- `nop` — advances PC without touching the stack or locals.
- `pop` — discards the top-of-stack value; raises `RuntimeError` on underflow.
- Bitwise and shift opcodes: `ishl` (left shift, masking shift amount to 5 bits),
  `ishr` (arithmetic right shift), `iand` (bitwise AND), `ior` (bitwise OR) — all
  routed through `_do_binary_op` with `_to_i32` overflow normalisation.
- `i2b` — truncates an int to its lowest 8 bits and sign-extends the result to a
  Python int in the range [-128, 127].

## [0.1.0] - 2026-03-18

### Added
- `JVMOpcode` enum with real JVM opcode values (iconst_0-5, bipush, ldc, iload/istore variants, iadd/isub/imul/idiv, goto, if_icmpeq, if_icmpgt, ireturn, return)
- `JVMTrace` dataclass capturing PC, opcode mnemonic, stack before/after, locals snapshot, and description
- `JVMSimulator` class with load(), step(), and run() methods
- Variable-width bytecode decoding matching real JVM encoding
- Constant pool support via ldc instruction
- Control flow: goto (unconditional), if_icmpeq (branch if equal), if_icmpgt (branch if greater)
- Helper functions: assemble_jvm(), encode_iconst(), encode_istore(), encode_iload()
- Comprehensive test suite with >80% coverage

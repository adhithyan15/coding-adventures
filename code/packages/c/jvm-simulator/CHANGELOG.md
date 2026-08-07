# Changelog

All notable changes to the C `jvm-simulator` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `jvm-simulator` crate.
- `JvmSimulator`: a typed stack-based VM over the int opcode subset
  (`iconst_0..5`, `bipush`, `ldc`, `iload`/`iload_0..3`, `istore`/`istore_0..3`,
  `iadd`/`isub`/`imul`/`idiv`, `if_icmpeq`/`if_icmpgt`/`goto`,
  `ireturn`/`return`).
- `jvm_sim_step` / `jvm_sim_run` producing a `JvmTrace` per instruction
  (stack before/after, a locals snapshot with per-slot initialized flags, and a
  description), with `jvm_trace_free` / `jvm_traces_free`.
- State accessors: `jvm_sim_stack`, `jvm_sim_locals`, `jvm_sim_pc`,
  `jvm_sim_halted`, `jvm_sim_return_value`.
- Bytecode assembler: `JvmProgram` + `jvm_emit` (1-byte or 2-byte big-endian
  operands per opcode).
- Faithful divergences from Rust: panics become `JvmStatus` codes;
  `idiv INT32_MIN / -1` is special-cased to `INT32_MIN` to avoid C UB.
- Allocations are guarded against `size_t` overflow (checked growth + multiply).
- 8 test groups (43 checks) mirroring the Rust crate's own tests, run under
  every available C compiler via the shared `iso-harness`.

# Changelog

All notable changes to the C++ `jvm-simulator` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `jvm-simulator` crate, in
  namespace `ca::jvm`.
- `JVMSimulator`: a typed stack-based VM over the int opcode subset
  (`iconst_0..5`, `bipush`, `ldc`, `iload`/`iload_0..3`, `istore`/`istore_0..3`,
  `iadd`/`isub`/`imul`/`idiv`, `if_icmpeq`/`if_icmpgt`/`goto`,
  `ireturn`/`return`).
- `step` / `run` producing a `JVMTrace` per instruction (stack before/after, a
  `std::optional`-slot locals snapshot, and a description).
- Bytecode assembler `assemble_jvm(Instr…)` plus `encode_iconst` /
  `encode_iload` / `encode_istore` helpers that choose the compact opcode form.
- Faithful divergences from Rust: panics become exceptions
  (`std::runtime_error` / `std::out_of_range`); `idiv INT32_MIN / -1` is
  special-cased to `INT32_MIN` to avoid C++ UB.
- 11 test groups (26 checks) mirroring the Rust crate's own tests plus the
  encode-helper forms, run under every available C++ compiler via the shared
  `iso-harness`.

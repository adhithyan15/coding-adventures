# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `x86_64-encoder` crate in
  namespace `ca::x86_64_encoder`: a stream-style x86-64 (AMD64) `Assembler` that
  emits little-endian machine-code bytes and resolves label-relative rel32
  branches at `finish()`.
- MOV family (reg/reg, imm32, movabs imm64, mem load/store with SIB, RIP-relative
  `lea`), integer arithmetic, logical, shifts, compare + `setcc` + `movzx`, SSE2
  scalar double + int⇄real conversions, stack, control flow, and misc; plus
  `ExternalReloc` recording for `lea`/`call_rel32`.
- Idiomatic C++ surface: `Reg`/`Cond` enums whose values are the register/
  condition codes, `LabelId`, and exceptions (`Error` carrying an `ErrorKind`)
  where the Rust crate returns `Result`. Verified clean under ASan + UBSan.
- 57 checks with byte-exact x86-64 encodings mirroring the crate's unit tests,
  run under every ISO C++ compiler via the shared `iso-harness`.

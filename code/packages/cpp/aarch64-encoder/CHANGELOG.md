# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `aarch64-encoder` crate in
  namespace `ca::aarch64_encoder`: a stream-style AArch64 (ARM64) `Assembler`
  that emits little-endian 32-bit instruction words and resolves label-relative
  branches at `finish()`.
- ~45 instruction encoders (moves, integer arithmetic register + immediate,
  `sdiv`/`udiv`/`msub`, logical, variable shifts, `neg`, compare, scaled
  loads/stores, scalar double-precision FP + int⇄real conversions, `stp`/`ldp`,
  branches, and misc) plus the label/fix-up branch resolver.
- Idiomatic C++ surface: `Reg`/`Cond` enums whose values are the register/
  condition codes, `LabelId`, and exceptions (`Error` carrying an `ErrorKind`)
  where the Rust crate returns `Result`. Verified clean under ASan + UBSan.
- 65 checks with byte-exact ARM64 encodings mirroring the crate's unit tests,
  run under every ISO C++ compiler via the shared `iso-harness`.

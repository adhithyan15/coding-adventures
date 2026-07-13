# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `x86_64-encoder` crate: a stream-style x86-64
  (AMD64) assembler that emits little-endian machine-code bytes (REX + opcode +
  ModR/M + SIB + disp/imm) in 64-bit mode and resolves label-relative rel32
  branches at finalisation.
- MOV family (reg/reg, imm32, movabs imm64, mem load/store with automatic SIB,
  RIP-relative `lea`), integer arithmetic (`add`/`sub`/`imul`/`idiv`/`div`/`cqo`/
  `neg`, imm32 forms), logical, shifts (CL + imm8), compare + `setcc` + `movzx`,
  SSE2 scalar double (`movsd`/`addsd`/…/`sqrtsd`) and int⇄real conversions, stack
  (`push`/`pop`), control flow (`jmp`/`jcc`/`call_*`/`ret`), and misc; plus
  external-relocation recording (`x64_external_reloc_*`) for `lea`/`call_rel32`.
- Opaque `X64Assembler` with a **sticky-error** model (`x64_error`) mirroring the
  Rust `Result` at every fallible step; `x64_finish` emits the malloc'd byte
  stream. All growable buffers guard `size_t` overflow. Verified clean under
  ASan + UBSan and the macOS `leaks` tool (0 leaks).
- 60 checks with byte-exact x86-64 encodings mirroring the crate's unit tests,
  run under every ISO C compiler via the shared `iso-harness`.

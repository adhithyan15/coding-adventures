# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `aarch64-encoder` crate: a stream-style AArch64
  (ARM64) assembler that emits little-endian 32-bit instruction words and
  resolves label-relative branches at finalisation.
- ~45 instruction encoders — moves (`movz`/`movk`/`mov_imm64`), integer
  arithmetic (register + 12-bit immediate, `sdiv`/`udiv`/`msub`), logical,
  variable shifts, `neg`, compare, scaled loads/stores (byte + double), scalar
  double-precision FP and int⇄real conversions, `stp`/`ldp`, branches
  (`b`/`bl`/`b.cond`/`cbz`/`cbnz`/`blr`/`ret`), and misc (`cset`/`nop`/`udf`/
  `svc`/`adrp` placeholder) — plus the label/fix-up resolver in `a64_finish`.
- Opaque `A64Assembler` with a **sticky-error** model (`a64_error`): a bad
  immediate or a re-bound label latches the first error, which `a64_finish`
  returns — mirroring the Rust `Result` at every fallible step. All growable
  buffers guard `size_t` overflow. Verified clean under ASan + UBSan and the
  macOS `leaks` tool (0 leaks).
- 64 checks with byte-exact ARM64 encodings mirroring the crate's unit tests,
  run under every ISO C compiler via the shared `iso-harness`.

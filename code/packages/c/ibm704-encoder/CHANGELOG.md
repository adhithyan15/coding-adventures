# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `ibm704-encoder` crate: an instruction encoder
  for the IBM 704 (1954), the mainframe on which Lisp was first run.
- `ibm704_encode_instruction` plus the named helpers `ibm704_encode_htr` /
  `ibm704_encode_cla` producing a 36-bit instruction word (opcode in bits
  35..27, address in bits 14..0; out-of-range address bits masked).
- `ibm704_pack_word` — the 5-byte little-endian wire packing — and the
  pre-computed `IBM704_HTR_HALT_BYTES` sentinel.
- Constants `IBM704_HTR`, `IBM704_CLA`, `IBM704_WORD_BITS`, `IBM704_WORD_MASK`,
  `IBM704_BYTES_PER_WORD`, `IBM704_ADDR_BITS`, `IBM704_ADDR_MASK`,
  `IBM704_OPCODE_SHIFT`.
- 21 checks mirroring the crate's doctests (McCarthy's canonical `CLA 42 ; HTR 0`
  program, the 36-bit word values, address masking, and the 5-byte packing), run
  under every ISO C compiler via the shared `iso-harness`; also clean under
  ASan + UBSan.

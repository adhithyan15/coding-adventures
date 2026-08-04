# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `ibm704-encoder` crate in
  namespace `ca::ibm704_encoder`: an instruction encoder for the IBM 704 (1954).
- `encode_instruction` plus `encode_htr` / `encode_cla` producing a 36-bit word,
  and `pack_word` returning the 5-byte little-endian wire form as a
  `std::array<std::uint8_t, 5>`; the `constexpr kHtrHaltBytes` sentinel.
- `constexpr` constants `kHtr`, `kCla`, `kWordBits`, `kWordMask`,
  `kBytesPerWord`, `kAddrBits`, `kAddrMask`, `kOpcodeShift`.
- 21 checks mirroring the crate's doctests (the canonical `CLA 42 ; HTR 0`
  program, word values, address masking, 5-byte packing), run under every ISO
  C++ compiler via the shared `iso-harness`.

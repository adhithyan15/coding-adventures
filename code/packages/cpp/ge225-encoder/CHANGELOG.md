# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `ge225-encoder` crate in
  namespace `ca::ge225_encoder`: an instruction encoder for the GE-225 (1959).
- `encode_lda` / `_sta` / `_ld` / `_add` / `_sub` / `_br` / `_bnz` / `_bz` /
  `_bmi` / `_jsr`, each returning a `std::array<std::uint8_t, 3>`;
  `decode_word` returning `std::pair<std::uint8_t, std::uint16_t>`.
- `constexpr` opcode-nibble constants, `kHaltWord` / `kRtsWord`, and the
  `kLda*` / `kGpRegisterCount` capacity constants.
- 23 checks mirroring the crate's doctests, run under every ISO C++ compiler via
  the shared `iso-harness`.

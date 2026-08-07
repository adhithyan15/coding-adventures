# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `intel4004-encoder` crate in
  namespace `ca::intel4004_encoder`: an instruction encoder for the Intel 4004
  (1971).
- `encode_ldm` / `encode_ld` / `encode_xch` (single byte) and `encode_jun`
  (returning a `std::array<std::uint8_t, 2>`); the `constexpr` `kHaltLoop`,
  opcode constants, and `kGpRegisterCount` / `kLdmMax` / `kLdmMinSigned`.
- 17 checks mirroring the crate's doctests, run under every ISO C++ compiler via
  the shared `iso-harness`.

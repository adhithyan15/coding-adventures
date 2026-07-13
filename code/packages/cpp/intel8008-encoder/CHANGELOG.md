# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `intel8008-encoder` crate in
  namespace `ca::intel8008_encoder`: an instruction encoder for the Intel 8008
  (1972), the second-generation 8-bit Intel microprocessor.
- `constexpr` `encode_mvi_a` (returning `std::array<std::uint8_t, 2>`) and
  `encode_jmp` / `encode_cal` (returning `std::array<std::uint8_t, 3>`, 14-bit
  address masked); the `constexpr` opcode constants `kHlt` / `kRet` / `kMviA` /
  `kJmp` / `kCal` and the `kGpRegisterCount` / `kMviMax` capacity constants.
- 16 checks mirroring the crate's doctests plus address-masking and a
  compile-time `static_assert`, run under every ISO C++ compiler via the shared
  `iso-harness`.

# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the Rust `armv7-encoder` crate, in
  namespace `ca::armv7`: a pure ARMv7-A (A32) instruction encoder.
- `constexpr` canonical word constants (`BX_LR`, `BKPT`, `MOV_IMM_R0_BASE`,
  `MOV_REG_BASE`), capacity constants (`GP_REGISTER_COUNT`, `MOV_IMM_MAX`), and
  the `constexpr` encoders `encode_mov_imm` / `encode_mov_reg`.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) assert exact ARM A32
  machine words, including compile-time `static_assert`s for the doc vectors.

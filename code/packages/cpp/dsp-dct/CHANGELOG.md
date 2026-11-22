# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Header-only ISO C++17 implementation (`include/dsp_dct.hpp`) in namespace `ca::dsp_dct` supporting 1-D DCT-I..IV, DST-I..IV, and separable 2-D DCT/IDCT and DST/IDST.
- O(N log N) Makhoul reduction for DCT-II and DCT-III in double precision with range-checked narrowing (`std::abs(v) <= FLT_MAX`).
- Alias-safe `dct_into` / `idct_into`, string parsers, `DctError` exceptions, and VER15 CWE hardening.
- Unit, hostile-input, and deterministic SplitMix64 PBT test suite via `iso-harness`.

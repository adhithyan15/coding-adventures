# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Header-only pure ISO C++17 port of `dsp-complex` (`namespace ca::dsp_complex`).
- `Complex` scalar type and `ComplexTensor` (`ComplexArray`) interleaved `[re, im]`
  container with `MAX_SAMPLES` (`2^24`) cap and overflow-checked `std::size_t` sizing.
- Checked constructors (`zeros`, `from_real`, `from_real_imag`, `from_interleaved`),
  safe bounds-checked `at` / `set`, projections (`real`, `imag`, `magnitude`, `phase`),
  and element-wise ops (`conjugate`, `scale`, `add`, `sub`, `mul`, `div`).
- Unit, hostile-input CWE, and deterministic SplitMix64 PBT tests via `iso-harness`.

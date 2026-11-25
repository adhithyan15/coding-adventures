# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Pure ISO C17 implementation of 1-D DCT-I..IV, DST-I..IV, and separable 2-D DCT/IDCT and DST/IDST (`dsp_dct.h`, `dsp_dct.c`) with zero `<math.h>` or `-lm` dependencies.
- O(N log N) Makhoul reduction for DCT-II and DCT-III using an internal iterative radix-2 + Bluestein FFT in double precision.
- Alias-safe in-place `*_into` variants (`out == signal`), string type/norm parsers, and VER15 CWE-20/190/400/476/674 hardening.
- Unit, hostile-input, and deterministic SplitMix64 PBT test suite via `iso-harness`.

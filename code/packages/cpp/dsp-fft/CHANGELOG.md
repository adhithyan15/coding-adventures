# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Header-only pure ISO C++17 port of `dsp-fft` (`namespace ca::dsp_fft`),
  built on `cpp/dsp-complex`.
- Iterative radix-2 Cooley-Tukey (`fft_scalar`, `ifft_scalar`), arbitrary-length
  Bluestein chirp-Z (`bluestein_scalar`), real-input half-spectrum transforms
  (`rfft_scalar`, `irfft_scalar`, `rfft`, `irfft`), and high-level `fft` / `ifft`.
- VER15 CWE hardening: `NaN`/`Inf`/odd/zero/non-pow2 rejection, checked
  `std::size_t` arithmetic, `MAX_FFT_LEN` (`2^20`) & `MAX_SAMPLES` (`2^24`) caps,
  and `nullptr` guards.
- Unit, DFT oracle, Family-1/4 invariant, hostile-input, and SplitMix64 PBT tests
  via `iso-harness`.

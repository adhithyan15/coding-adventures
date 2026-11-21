# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Pure ISO C17 port of `dsp-fft` with zero `<math.h>` / `-lm` dependency.
- Iterative decimation-in-time radix-2 Cooley-Tukey FFT/IFFT (`dsp_fft_scalar`,
  `dsp_ifft_scalar`) and arbitrary-length Bluestein chirp-Z FFT/IFFT
  (`dsp_bluestein_scalar`).
- Real-input half-spectrum transforms (`dsp_rfft_scalar`, `dsp_irfft_scalar`)
  and `dsp_complex_tensor_t` entry points (`dsp_fft`, `dsp_ifft`, `dsp_rfft`,
  `dsp_irfft`).
- VER15 CWE hardening: `NaN`/`Inf`/odd/zero/non-pow2 rejection, checked `size_t`
  arithmetic, `DSP_FFT_MAX_LEN` (`2^20`) & `DSP_FFT_MAX_SAMPLES` (`2^24`) caps,
  NULL guards, and zero-on-free temporary and output buffer cleanup.
- Unit, DFT oracle, Family-1/4 invariant, hostile-input, and SplitMix64 PBT tests
  via `iso-harness`.

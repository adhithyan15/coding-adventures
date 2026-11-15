# Changelog

All notable changes to `coding-adventures-dsp-fft` will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Pure Python iterative Cooley-Tukey radix-2 FFT and IFFT.
- Arbitrary-$N$ Bluestein chirp Z-transform FFT and IFFT.
- Real-input half-spectrum `rfft` and `irfft`.
- High-level `fft` and `ifft` dispatching seamlessly to `dsp_complex.ComplexTensor`.
- Pre-flight CWE-400 bounds checks (`MAX_FFT_LEN = 1_048_576`, `MAX_SAMPLES = 16_777_216`).
- CWE-20 finite-float validation rejecting `NaN` and `Inf`.
- Warrant specification `dsp-fft.warrant` and required capabilities manifest.

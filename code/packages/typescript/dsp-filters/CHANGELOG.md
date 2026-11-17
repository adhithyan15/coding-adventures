# Changelog

## Unreleased

### Fixed
- `dsp-filters.warrant` was invalid Warrant (`max` is not a builtin); the iir steps bound now uses `imax`. Document only.

## 0.1.0

- Initial release of `@coding-adventures/dsp-filters`.
- `Biquad` Audio EQ Cookbook implementation with Direct-Form II Transposed structure.
- `lfilter` and `iir` Direct-Form II Transposed IIR filtering.
- `sosfilt` Second-Order Sections cascade.
- `filtfilt` zero-phase forward-backward filtering.
- Direct FIR filtering (`fir`) and FFT-accelerated fast convolution (`firFft` / `fir_fft`).
- Classical filter design: `firwin`, `designBandPass`, `designLowPass`, `designHighPass`, `butterworthLowpass`, `butterworthHighpass`, `butterworthBandpass`, `chebyshev1Lowpass`.
- VER15 security hardening: CWE-20, CWE-400, CWE-674, CWE-1321, CWE-1333 compliance.

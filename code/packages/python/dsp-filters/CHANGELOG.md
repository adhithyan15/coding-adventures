# Changelog

All notable changes to `coding-adventures-dsp-filters` will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Pure Python FIR filtering via direct convolution (`fir`) and FFT convolution (`fir_fft`).
- IIR filtering via Direct-Form-II Transposed structure (`iir`).
- Windowed-sinc FIR design helpers (`design_low_pass`, `design_high_pass`, `design_band_pass`).
- Butterworth lowpass IIR design (`butterworth_lowpass`).
- Chebyshev Type I lowpass IIR design (`chebyshev1_lowpass`).
- Second-order sections (biquad) and SOS cascade filtering (`Biquad`, `sos_cascade`, `sosfilt`).
- Pre-flight CWE-400 bounds checks (`MAX_SAMPLES = 16_777_216`, `MAX_TAPS = 1_048_576`, `MAX_ORDER = 1024`).
- CWE-20 finite-float enforcement rejecting `NaN` and `Inf`.
- Warrant specification `dsp-filters.warrant` and required capabilities manifest.

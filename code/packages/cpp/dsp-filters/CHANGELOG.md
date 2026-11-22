# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Header-only ISO C++17 implementation (`include/dsp_filters.hpp`) in namespace `ca::dsp_filters`.
- FIR (`fir`, `fir_fft`), windowed-sinc FIR design (`low_pass`, `high_pass`, `band_pass`, `band_stop`), Direct-Form-II Transposed IIR (`iir`, `filtfilt`), failure-atomic `Biquad` with Audio EQ Cookbook constructors, SOS (`sosfilt`, `sosfiltfilt`), and Butterworth LP/HP/BP/BS design (`butterworth_sos` and `(b, a)` wrappers).
- Alias-safe `*_into` variants (`out == signal`), `FilterError` exceptions, and VER15 CWE hardening.
- Unit, hostile-input, and deterministic SplitMix64 PBT test suite via `iso-harness`.

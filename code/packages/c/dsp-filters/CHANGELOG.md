# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Pure ISO C17 implementation (`dsp_filters.h`, `dsp_filters.c`) with zero `<math.h>` or `-lm` dependencies.
- FIR (`dsp_fir`, `dsp_fir_fft`) and windowed-sinc FIR design (`low_pass`, `high_pass`, `band_pass`, `band_stop`).
- Direct-Form-II Transposed IIR (`dsp_iir`, `dsp_filtfilt`), failure-atomic `dsp_biquad_t` with Audio EQ Cookbook constructors, Second-Order Sections (`dsp_sosfilt`, `dsp_sosfiltfilt`), and Butterworth LP/HP/BP/BS design (`dsp_butterworth_sos` and `(b, a)` wrappers).
- Alias-safe `*_into` variants (`out == signal`) and VER15 CWE-20/190/400/476/674 hardening.
- Unit, hostile-input, and deterministic SplitMix64 PBT test suite via `iso-harness`.

# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Header-only, self-contained pure ISO C++17 (zero `<cmath>` / `-lm`) implementation of `dsp-stft` in `ca::dsp_stft`.
- Internal $O(N \log N)$ radix-2 + Bluestein FFT and internal $O(N \log N)$ Makhoul DCT-II supporting arbitrary frame sizes and Mel counts.
- Forward STFT (`stft`), overlap-add ISTFT (`istft`), OLA/COLA verification (`check_ola`), and iterative Griffin-Lim (`griffin_lim`).
- Magnitude, power, and log-power spectrograms; HTK Mel scale (`hz_to_mel`, `mel_to_hz`), triangular Mel filterbank (`mel_filterbank`), Mel spectrogram (`mel_spectrogram`), and MFCC (`mfcc`).
- VER15 & Zero-CVE hardening with unit, Family-1/4 invariant, hostile-input, and deterministic SplitMix64 PBT suites.

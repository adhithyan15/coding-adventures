# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-11-21

### Added

- Self-contained pure ISO C17 (zero `<math.h>` / `-lm`) implementation of `dsp-stft`.
- Internal $O(N \log N)$ radix-2 + Bluestein FFT and internal $O(N \log N)$ Makhoul DCT-II supporting arbitrary frame sizes and Mel counts.
- Forward STFT (`stft_forward`), overlap-add ISTFT (`stft_istft`), OLA/COLA verification (`stft_check_ola`), and iterative Griffin-Lim (`stft_griffin_lim`).
- Magnitude, power, and log-power spectrograms; HTK Mel scale (`stft_hz_to_mel`, `stft_mel_to_hz`), triangular Mel filterbank (`stft_mel_filterbank`), Mel spectrogram (`stft_mel_spectrogram`), and MFCC (`stft_mfcc`).
- VER15 & Zero-CVE hardening with unit, Family-1/4 invariant, hostile-input, and deterministic SplitMix64 PBT suites.

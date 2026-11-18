# Changelog

## Unreleased

### Tests
- Declare `test-bootstrap` as a development dependency and install it in both build recipes so the PBT suite uses the shared `SplitMix64` implementation.

## 0.1.0 - 2026-11-15

### Added
- `build_window` and `WindowType` (`Rectangular`, `Hann`, `Hamming`, `Blackman`).
- `stft` strict-mode short-time Fourier transform.
- `istft` inverse short-time Fourier transform via overlap-add with NOLA/COLA reconstruction.
- `spectrogram`, `power_spectrogram`, and `log_spectrogram`.
- `hz_to_mel` and `mel_to_hz` (HTK convention).
- `mel_filterbank` triangular filterbank construction.
- `mel_spectrogram` mel-scale spectral pooling.
- `mfcc` Mel-Frequency Cepstral Coefficients via DCT-II.
- VER15 security hardening against CWE-400, CWE-20, CWE-674, CWE-1333, and pure sandbox capabilities.

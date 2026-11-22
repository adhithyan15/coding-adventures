# Changelog

## Unreleased

### Added
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`) into `tests/pbt.test.ts` and updated `BUILD` / `BUILD_windows` in leaf-to-root dependency order.
- Hardened VER15 input boundaries with `safeFormatValue` (safe formatting for `Symbol` and `Object.create(null)`), `validateSequence` (`ArrayBuffer.isView` / `DataView` / `BigInt64Array` / `BigUint64Array` / `null` / non-safe-integer `.length` pre-flight bounds), safe-integer output bounds, and pre-flight `length > 64` enum string guards before `.trim().toLowerCase()`, with hostile regression tests.

### Fixed
- dsp-stft.warrant was invalid Warrant (`/` in bound expressions); rewritten with `div`. Document only.

## 0.1.0

- Initial release of `@coding-adventures/dsp-stft`.
- Forward STFT (`stft`) and Inverse STFT (`istft`).
- Power and log spectrograms (`spectrogram`, `powerSpectrogram`, `logSpectrogram`).
- Analysis windows (`Hann`, `Hamming`, `Blackman`, `Rectangular`).
- Perceptual speech features (`melFilterbank`, `melSpectrogram`, `mfcc`).
- Complete VER15 security hardening with strict bounds and non-finite float rejection.

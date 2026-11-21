# Changelog

## Unreleased

### Added
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`) into `tests/pbt.test.ts` and updated `BUILD` / `BUILD_windows` in leaf-to-root dependency order.
- Hardened VER15 input boundaries with `safeFormatValue` (safe formatting for `Symbol` and `Object.create(null)`), `validateSequence` (`ArrayBuffer.isView` / `DataView` / `BigInt64Array` / `BigUint64Array` / `null` / non-safe-integer `.length` pre-flight bounds), and pre-flight `length > 64` enum string guards before `.trim().toLowerCase()`, with hostile regression tests.

## 0.1.0

- Initial release of `@coding-adventures/dsp-conv`.
- 1-D convolution (`conv1d`, `convolve`, `correlate`).
- FFT convolution (`fftConvolve`) and overlap-add block convolution (`overlapAdd`).
- 2-D spatial and separable convolution (`conv2d`, `sepConv2d`, `convolve2d`).
- Kernel generators for Gaussian, box, Sobel, Laplacian, and sharpen filters.
- Complete VER15 security hardening with fail-closed bounds and finite-float verification.

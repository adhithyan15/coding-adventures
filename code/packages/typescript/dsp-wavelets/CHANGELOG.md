# Changelog

## Unreleased

### Added
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`) into `tests/pbt.test.ts` and updated `BUILD` / `BUILD_windows` in leaf-to-root dependency order.
- Hardened VER15 input boundaries with `safeFormatValue` (safe formatting for `Symbol` and `Object.create(null)`), `validateSequence` (`ArrayBuffer.isView` / `DataView` / `BigInt64Array` / `BigUint64Array` / `null` / non-safe-integer `.length` pre-flight bounds), `Map`-based `Wavelet.db` filter lookup (closing prototype-key injection), and pre-flight `length > 64` enum/wavelet string guards before `.trim().toLowerCase()`, with hostile regression tests.

## 0.1.0

- Initial release of `@coding-adventures/dsp-wavelets`.
- 1-D DWT and IDWT (`dwt`, `idwt`, `dwt1d`, `idwt1d`).
- 2-D DWT and IDWT (`dwt2d`, `idwt2d`).
- Multi-level decomposition (`wavedec`, `waverec`).
- Stationary Wavelet Transform (`swt`, `iswt`).
- Continuous Wavelet Transform (`cwt`) for Morlet and Ricker wavelets.
- Complete VER15 security hardening with strict bounds and non-finite float rejection.

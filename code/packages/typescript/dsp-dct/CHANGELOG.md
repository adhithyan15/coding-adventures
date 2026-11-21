# Changelog

## Unreleased

### Added
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`, `checkProperty`, `fuzzTarget`) into `package.json`, `BUILD`, `BUILD_windows`, and `tests/pbt.test.ts` (DCT-I..IV / DST-I..IV round-trip & Parseval's theorem, linearity, 2-D DCT/IDCT round-trip & Parseval, and CWE-20/190/400/1321 fuzzing).
- Added VER15 hostile regression tests for `Number.isSafeInteger` (`1e100`), `Symbol`/`Object.create(null)` error formatting & element decoding, `DataView`/`BigInt64Array`/oversized typed arrays, and CWE-1321 prototype pollution resistance.
- Added `coverage/` to `.gitignore`.

### Fixed
- Hardened `errors.ts`, `types.ts`, `dct.ts`, `dst.ts`, and `two_d.ts` with `Number.isSafeInteger`, `safeFormatValue`, `validateArrayLikeLength` (preventing `DataView` and negative/fractional length crashes), `readFiniteNumber`, and >64-char enum string bounds before `.trim().toLowerCase()` in `parseNorm`, `parseType`, and `parseDstType`.

## 0.1.0

- Initial release of `@coding-adventures/dsp-dct`.
- Full DCT-I..IV with O(N log N) Makhoul FFT reductions for DCT-II and DCT-III.
- DST-I..IV with orthonormal and unnormalized modes.
- 2-D separable DCT and IDCT (`dct2d`, `idct2d`).
- VER15 security hardening: CWE-20, CWE-400, CWE-674, CWE-1321, CWE-1333 compliance.

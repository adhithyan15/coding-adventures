# Changelog

## Unreleased

### Added
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`, `checkProperty`, `fuzzTarget`) into `package.json`, `BUILD`, `BUILD_windows`, and `tests/pbt.test.ts` (complex scalar & array field axioms plus CWE-20/190/400/1321 fuzzing).
- Added VER15 hostile regression tests for `Number.isSafeInteger` (`1e100`), `Symbol`/`Object.create(null)` error formatting & element decoding, `DataView`/`BigInt64Array`/oversized typed arrays, and CWE-1321 prototype pollution resistance.
- Added `coverage/` to `.gitignore` and `--test-coverage-exclude=../**` to test scripts.

### Fixed
- Hardened `errors.ts`, `complex.ts`, and `complex_array.ts` with `Number.isSafeInteger`, `safeFormatValue`, `validateArrayLikeLength` (preventing `DataView` silent 0-length allocation and negative/fractional length `RangeError`s), and `readFiniteNumber`.
- Replaced naive complex division with Smith's scaled complex division in `Complex.div` and `ComplexArray.div` (preventing `1e200 / 1e200 -> NaN` squaring overflow) and enforced `Number.isFinite` on every element computed by `ComplexArray.{scale,add,sub,mul,div}`.

## 0.1.0

- Initial release of `@coding-adventures/dsp-complex`.
- `Complex` immutable scalar complex type with arithmetic, polar conversion, and trigonometric/exponential operations.
- `ComplexArray` / `ComplexTensor` contiguous interleaved Float64Array representation.
- VER15 security hardening: CWE-20, CWE-400, CWE-674, CWE-1321, CWE-1333 compliance.

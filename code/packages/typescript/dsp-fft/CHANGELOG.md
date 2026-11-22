# Changelog

## Unreleased

### Added
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`, `checkProperty`, `fuzzTarget`) into `package.json`, `BUILD`, `BUILD_windows`, and `tests/pbt.test.ts` (Radix-2/Bluestein FFT/IFFT round-trip, Parseval's theorem, linearity, `rfft`/`irfft` round-trip, window symmetry, and CWE-20/190/400/1321 fuzzing).
- Added VER15 hostile regression tests for `Number.isSafeInteger` (`1e100`), 32-bit `&` truncation (`2^32 + 1`), `Symbol`/`Object.create(null)`, non-string window types, `DataView`/`BigInt64Array`/oversized typed arrays, and CWE-1321 prototype pollution resistance.
- Added `coverage/` to `.gitignore`.

### Fixed
- Hardened `errors.ts`, `radix2.ts`, `bluestein.ts`, `fft.ts`, `rfft.ts`, and `windows.ts` with `Number.isSafeInteger`, `MAX_FFT_LEN` guard before 32-bit `&` bitmask in `requirePow2Length`, `safeFormatValue`, `validateArrayLikeLength`, `readFiniteNumber`, fail-closed non-string and >64-char string rejection in `parseWindowType`, and `MAX_KAISER_BETA = 700.0` / 1024-iteration convergence + finiteness guard in `besselI0` & `kaiser`.

## 0.1.0

- Initial release of `@coding-adventures/dsp-fft`.
- Radix-2 Cooley-Tukey iterative forward and inverse FFT.
- Bluestein chirp Z-transform arbitrary-length FFT.
- Real-valued `rfft` and `irfft` half-spectrum transforms.
- Window functions: Hann, Hamming, Blackman, and Kaiser.
- High-level `ComplexArray` / `ComplexTensor` integration.
- VER15 security hardening: CWE-20, CWE-400, CWE-674, CWE-1321, CWE-1333 compliance.

# Changelog

## Unreleased

### Added
- Wired `@coding-adventures/test-bootstrap` (`SplitMix64`, `checkProperty`, `fuzzTarget`) into `tests/pbt.test.ts`, `package.json`, `BUILD`, and `BUILD_windows`.
- Added VER15 hostile regression tests for `Number.isSafeInteger` (`1e100`), `Symbol`/`Object.create(null)` error formatting, `ArrayBuffer.isView` (`DataView`, `BigInt64Array`, oversized typed arrays), and CWE-1321 prototype pollution resistance.
- Added `coverage/` to `.gitignore`.

### Fixed
- Hardened `validation.ts`, `prng.ts`, `sampler.ts`, and `state.ts` with `Number.isSafeInteger`, `safeFormatValue` (preventing `String(Object.create(null))` / `Symbol` crashes), `DataView` / `ArrayBuffer.isView` guards, and `null`/non-object `config` guards in `OscillatorState` and `synthesize()`.

## 0.2.0 - 2026-11-16

### Changed
- `SineOscillator` / `evaluateSineRaw` take their sine from the bit-exact `trig` package (TRIG-REF-1, PHY00a) instead of `Math.sin`, so the same bits come out in Python, Rust and TypeScript. The phase is reduced to [0, 1) cycles (`phase - floor(phase)`, exact) first, because `trig.sin` is NaN beyond 2^29 radians (a 440 Hz tone passes that after about 54 hours).
- New dependency `trig` (`file:../trig`); BUILD files build it first.

## 0.1.0 - 2026-11-15

### Added
- Initial TypeScript implementation of OSC00 (`@coding-adventures/oscillator`).
- Pure continuous waveforms (`SineOscillator`, `SquareOscillator`, `TriangleOscillator`, `SawtoothOscillator`, `PulseOscillator`).
- Stateful phase accumulator (`OscillatorState`) with `step()`, `renderSamples()`, and `synthesize()`.
- Deterministic PRNG white noise engines (`SplitMix64`, `LcgPrng`).
- Sampling primitives (`UniformSampler`, `SampleBuffer`) with half-open interval semantics.
- Security hardening for CWE-20 and CWE-400 with pre-flight bounds checks.
- Warrant conformance operations `calcNyquist` and `evalSquareDuty`.

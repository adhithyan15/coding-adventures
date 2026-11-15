# Changelog

## 0.1.0 - 2026-11-15

### Added
- Initial TypeScript implementation of OSC00 (`@coding-adventures/oscillator`).
- Pure continuous waveforms (`SineOscillator`, `SquareOscillator`, `TriangleOscillator`, `SawtoothOscillator`, `PulseOscillator`).
- Stateful phase accumulator (`OscillatorState`) with `step()`, `renderSamples()`, and `synthesize()`.
- Deterministic PRNG white noise engines (`SplitMix64`, `LcgPrng`).
- Sampling primitives (`UniformSampler`, `SampleBuffer`) with half-open interval semantics.
- Security hardening for CWE-20 and CWE-400 with pre-flight bounds checks.
- Warrant conformance operations `calcNyquist` and `evalSquareDuty`.

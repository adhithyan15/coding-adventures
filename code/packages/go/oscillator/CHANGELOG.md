# Changelog

## 0.2.0 - 2026-11-16

- `SineOscillator.ValueAt` now reduces its phase to a fractional cycle
  (`phase - floor(phase)`, exact in binary64) before converting to radians. The
  `go/trig` it depends on now returns NaN beyond 2^29 radians (TRIG-REF-1, PHY00a
  3.4.1), and a 440 Hz tone passes that after about 54 hours; before this change
  the phase `2*pi*(f*t + phase_cycles)` was handed to `trig.Sin` unreduced. This
  is the same fix, with the same bits, as the Python, Rust and TypeScript
  oscillators (TC-2).
- Every product that feeds an addition (`f*t + phase`, `offset + amplitude*sin`,
  the sample-count product) is wrapped in `float64()` so Go cannot fuse it into an
  FMA on arm64, matching the other languages bit for bit.
- A phase that overflows to infinity yields NaN (not an error), like the other
  languages.
- Tests: a week-old 440 Hz sine, exact periodicity in cycles at `t = 1e9`, seven
  cross-language bit vectors from the Python oscillator, and the NaN case.

## 0.1.0 - 2026-04-20

- Added the initial virtual oscillator package for Go.
- Added `ContinuousSignal`, `SineOscillator`, `SquareOscillator`,
  `UniformSampler`, and `SampleBuffer`.
- Added helper functions for sample counts, sample times, and Nyquist
  frequency.
- Added parity tests from `OSC00: Oscillator and Sampler`.

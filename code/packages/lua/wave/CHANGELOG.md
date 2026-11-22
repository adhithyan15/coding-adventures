# Changelog — coding-adventures-wave (Lua)

## [0.2.0] - 2026-11-16

### Fixed

- `sine_wave` and `cosine_wave` reduce the phase to a fractional cycle
  (`cycles - floor(cycles)`) and the phase offset with `math.fmod` before calling `trig`,
  instead of passing `2*pi*f*t + phase` unreduced. `trig` 0.3.0 (TRIG-REF-1) returns NaN
  beyond 2^29 radians, which the old angle reached after about 8.5e7 cycles (a 440 Hz
  tone after roughly 53 hours of samples). The same fix TC-2 applied to the Python, Rust
  and TypeScript music packages; the answer is also more accurate at every time.

### Changed

- Requires `coding-adventures-trig >= 0.3.0`; the rockspec is renamed
  `coding-adventures-wave-0.2.0-1.rockspec`. Samples may differ from 0.1.0 in the last
  bits (new `trig`).

### Tests

- Very large cycle counts (a 1e9 Hz tone sampled at 1 Hz, and a 1e10 rad phase offset)
  stay finite and correct for `sine_wave` and `cosine_wave`.

## Unreleased

### Added

- Add the source-compatible `wave.Wave` continuous PHY01 object API alongside
  the existing sampled-wave helpers.
- Validate finite parameters, angular-frequency overflow, and evaluation time;
  reduce time and phase; and cover maximum-finite and subnormal boundaries.

## [0.1.0] — 2026-03-29

### Added

- `sine_wave(frequency, amplitude, phase, sample_rate, num_samples)` — generates
  a sampled sine wave using `trig.sin_approx` from first principles.
- `cosine_wave(frequency, amplitude, phase, sample_rate, num_samples)` — generates
  a sampled cosine wave using `trig.cos_approx`.
- `square_wave(frequency, amplitude, sample_rate, num_samples)` — generates a
  50%-duty-cycle square wave (±amplitude, based on sign of sine).
- `sawtooth_wave(frequency, amplitude, sample_rate, num_samples)` — linear ramp
  using the floor-centering formula `2*amp*(t*f - floor(t*f + 0.5))`.
- `triangle_wave(frequency, amplitude, sample_rate, num_samples)` — triangle
  derived from a double-frequency sawtooth via absolute value and rescaling.
- `dc_offset(value, num_samples)` — constant-value signal array.
- `add_waves(wave1, wave2)` — element-wise sum of two equal-length arrays.
- `scale_wave(wave, scalar)` — multiply every sample by a scalar.
- `mix_waves(waves)` — element-wise sum of an arbitrary list of equal-length arrays.
- `TWO_PI` constant (re-exported from trig).
- Comprehensive test suite (`tests/test_wave.lua`) covering all functions with
  analytical spot-checks, Pythagorean identity, destructive interference,
  range checks, and error-condition tests.

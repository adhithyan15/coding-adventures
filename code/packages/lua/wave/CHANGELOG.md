# Changelog — coding-adventures-wave (Lua)

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

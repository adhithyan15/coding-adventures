# Changelog — CodingAdventures::Wave (Perl)

## [0.01] — 2026-03-29

### Added

- `sine_wave($freq, $amp, $phase, $sr, $n)` — sampled sine wave using
  `CodingAdventures::Trig::sin_approx` from first principles.
- `cosine_wave($freq, $amp, $phase, $sr, $n)` — sampled cosine wave.
- `square_wave($freq, $amp, $sr, $n)` — 50%-duty-cycle square wave
  (±amplitude based on sign of sine).
- `sawtooth_wave($freq, $amp, $sr, $n)` — linear ramp using the floor-centering
  formula `2*amp*(t*f - floor(t*f + 0.5))`.
- `triangle_wave($freq, $amp, $sr, $n)` — triangle wave derived from a
  double-frequency sawtooth via absolute value and rescaling.
- `dc_offset($value, $n)` — constant-value signal using Perl's `x` operator.
- `add_waves(\@w1, \@w2)` — element-wise sum with length mismatch check.
- `scale_wave(\@wave, $scalar)` — scalar multiplication via `map`.
- `mix_waves([\@w1, \@w2, ...])` — iterative sum of multiple waves.
- `$TWO_PI` constant (re-exported from Trig).
- Test suite (`t/00-load.t`, `t/01-basic.t`) covering all functions with
  analytical spot-checks, Pythagorean identity, destructive interference,
  range checks, and error-condition tests.

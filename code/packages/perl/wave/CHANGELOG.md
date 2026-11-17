# Changelog — CodingAdventures::Wave (Perl)

## [0.02] - 2026-11-16

### Fixed

- `sine_wave` and `cosine_wave` kept an unbounded angle (`2*pi*f*t + phase`) and
  passed it straight to `sin_approx` / `cos_approx`. Since the TRIG-REF-1
  convergence of `trig` (0.3.0, PHY00a TC-15) those return NaN beyond 2^29
  radians (about 8.5e7 cycles; a 440 Hz tone reaches it after about 53 hours of
  samples), so a long enough run would have turned every sample into NaN. The
  phase is now kept in cycles and reduced to its fractional cycle with
  `x - floor(x)` (`POSIX::floor`) before the call: `angle = 2*pi * (cycles -
  floor(cycles))`, `cycles = f*t + phase/(2*pi)`. The reduction is exact in
  binary64. Short runs give the same samples as before to within a few ulps.
  `CodingAdventures::Wave->new(...)->evaluate` already reduced time and phase and
  is unchanged.

### Tests

- Cover cycle counts far beyond 2^29 radians for `sine_wave` and `cosine_wave`,
  a huge phase, and the pi/2 phase identity.

### Carried from Unreleased

### Changed

- `BUILD_windows` now runs the real test suite on Windows through the shared `code/scripts/run-perl-tests-windows.ps1` helper (native Strawberry Perl, absolute `-I` paths for sibling libraries) instead of the previous skip or bare-`perl` recipe.

### Added

- Add a source-compatible continuous PHY01 object API alongside all existing
  exported sampled-wave helpers.
- Validate finite parameters, angular-frequency overflow, and time; reduce time
  and phase; and test maximum-finite and minimum-subnormal boundaries.

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

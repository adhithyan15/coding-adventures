# wave

A pure Haskell simple-harmonic wave model:

```text
y(t) = amplitude * sin(2 * pi * frequency * t + phase)
```

## API

- `newWave` constructs a zero-phase wave.
- `newWaveWithPhase` constructs a wave with an explicit phase.
- `amplitude`, `frequency`, and `phase` inspect the validated parameters.
- `period`, `angularFrequency`, and `evaluate` derive wave quantities.

Construction rejects negative amplitude and non-positive frequency. A zero
amplitude is valid and produces a flat wave. Evaluation uses the repository's
first-principles Haskell `trig` package rather than Prelude trigonometry.
It also rejects non-finite parameters, angular-frequency overflow, and
non-finite time; performs full-range binary64 time and phase reduction; and
keeps accepted extreme results finite and amplitude-bounded. Positive
subnormal frequencies remain valid when their represented period is infinite.

## Running the tests

```sh
cabal test all
```

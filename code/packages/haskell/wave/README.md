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

## Running the tests

```sh
cabal test all
```

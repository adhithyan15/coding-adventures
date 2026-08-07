# wave

Pure F# immutable sinusoidal wave model for signal-processing foundations.

The PHY01 contract rejects non-finite parameters, angular-frequency overflow,
and non-finite evaluation time. Evaluation reduces time and phase before using
the local first-principles `trig` package, preserves exact zero amplitude, and
stays finite and amplitude-bounded for every accepted binary64 input.

## What It Includes

- Amplitude, frequency, and phase validation
- Period and angular-frequency helpers
- Time-domain evaluation with `y(t) = A * sin(2*pi*f*t + phase)`

## Example

```fsharp
open CodingAdventures.Wave.FSharp

let wave = Wave(amplitude = 1.0, frequency = 440.0)
let sample = wave.Evaluate 0.001
```

## Development

```bash
bash BUILD
```

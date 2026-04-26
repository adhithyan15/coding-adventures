# wave

Pure C# immutable sinusoidal wave model for signal-processing foundations.

## What It Includes

- Amplitude, frequency, and phase validation
- Period and angular-frequency helpers
- Time-domain evaluation with `y(t) = A * sin(2*pi*f*t + phase)`

## Example

```csharp
using CodingAdventures.Wave;

var wave = new Wave(amplitude: 1.0, frequency: 440.0);
var sample = wave.Evaluate(0.001);
```

## Development

```bash
bash BUILD
```

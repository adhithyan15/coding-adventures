# wave

**Layer:** PHY01 (Physics Layer 1)
**Language:** Go
**Depends on:** [trig](../trig/) (MATH01)

## Overview

The `wave` package models sinusoidal waves — the mathematical foundation of sound, light, radio, and countless other physical phenomena. A sinusoidal wave is described by three parameters:

- **Amplitude** — the peak displacement from equilibrium
- **Frequency** — how many cycles occur per second (Hz)
- **Phase** — the initial angular offset (radians)

The core equation is:

```
y(t) = A * sin(2 * pi * f * t + phase)
```

This package uses the `trig` package for sine computation, maintaining a full chain of understanding from Taylor series to wave physics.

The full PHY01 binary64 contract rejects non-finite parameters,
angular-frequency overflow, and non-finite time. `Evaluate` revalidates the
public fields, short-circuits exact zero amplitude, reduces time and phase, and
keeps every accepted result finite and amplitude-bounded. A positive subnormal
frequency remains valid even if `Period` returns positive infinity.

## API

### Creating a Wave

```go
import "github.com/adhithyan15/coding-adventures/code/packages/go/wave"

// A 440 Hz tone (concert A) with unit amplitude
w, err := wave.New(1.0, 440.0, 0.0)
if err != nil {
    // handle error (negative amplitude or non-positive frequency)
}
```

### Evaluating at a Point in Time

```go
value := w.Evaluate(0.25) // displacement at t = 0.25 seconds
```

### Derived Properties

```go
w.Period()           // 1/frequency — time for one full cycle
w.AngularFrequency() // 2*pi*frequency — radians per second
```

### Validation

- `New()` returns `ErrNegativeAmplitude` if amplitude < 0
- `New()` returns `ErrZeroFrequency` if frequency <= 0

## How It Fits in the Stack

```
PHY01: wave (this package)
  └── MATH01: trig (sine from Maclaurin series)
```

The wave package is the first physics layer, demonstrating how mathematical primitives compose into physical models.

## Building

```bash
go test ./... -v -cover
```

Or use the project build tool, which handles dependency ordering automatically.

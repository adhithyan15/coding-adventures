# Wave Physics

**Layer:** PHY01 (Physics Layer 1)
**Depends on:** `coding-adventures-trig`

Simple harmonic wave model for electromagnetic wave fundamentals.

## The Wave Equation

A simple harmonic wave is the most fundamental waveform in physics:

```
y(t) = A * sin(2 * pi * f * t + phi)
```

where:

- **A** — amplitude (peak displacement from zero)
- **f** — frequency in Hertz (cycles per second)
- **t** — time in seconds
- **phi** — phase offset in radians

Every electromagnetic wave — light, radio, X-rays — can be decomposed into simple harmonic components via Fourier analysis.

## API

```python
from wave_physics import Wave

# Create a 440 Hz wave (concert A) with amplitude 1
w = Wave(amplitude=1.0, frequency=440.0)

# Evaluate at a specific time
value = w.evaluate(0.25)

# Derived quantities
w.period()             # 1/440 seconds (~2.27 ms)
w.angular_frequency()  # 2 * PI * 440 rad/s

# Phase-shifted wave (starts at peak)
from trig import PI
w2 = Wave(amplitude=1.0, frequency=440.0, phase=PI / 2)
w2.evaluate(0.0)  # 1.0 (starts at peak)
```

## Properties

| Property | Type | Description |
|----------|------|-------------|
| `amplitude` | float | Peak displacement (>= 0) |
| `frequency` | float | Cycles per second (> 0) |
| `phase` | float | Phase offset in radians |

## Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `evaluate(t)` | float | Wave value at time t |
| `period()` | float | Time for one full cycle (1/f) |
| `angular_frequency()` | float | Radians per second (2*pi*f) |

## Dependencies

This package imports `sin` and `PI` from the `trig` package rather than Python's `math` module, demonstrating how layers build on each other in the educational stack.

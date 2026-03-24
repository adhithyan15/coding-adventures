# Wave

**Layer:** PHY01 (Physics Layer 1)
**Depends on:** `coding-adventures-trig`

Simple harmonic wave model from first principles.

## The Wave Equation

A simple harmonic wave is described by:

```python
y(t) = A * sin(2 * pi * f * t + phi)
```

where:

- **A** is amplitude, the peak displacement from zero
- **f** is frequency in Hertz, the number of cycles per second
- **t** is time in seconds
- **phi** is phase offset in radians

This package focuses on the time-domain behavior of a sinusoidal wave, which
is the building block for sound, light, radio, and many other physical
phenomena.

## API

```python
from trig import PI
from coding_adventures_wave import Wave

w = Wave(amplitude=1.0, frequency=440.0)
w.evaluate(0.0)
w.period()
w.angular_frequency()

shifted = Wave(amplitude=1.0, frequency=440.0, phase=PI / 2)
shifted.evaluate(0.0)
```

## Methods

- `evaluate(t)` returns the wave value at time `t`
- `period()` returns `1 / frequency`
- `angular_frequency()` returns `2 * pi * frequency`

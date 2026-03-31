# coding-adventures-wave (Lua)

Signal and waveform generation library — generates digital representations of
periodic waveforms from first principles.

## Overview

This package produces arrays of floating-point samples representing classic
waveforms used in audio synthesis and digital signal processing. It depends on
`coding-adventures-trig` for trigonometric calculations, keeping the
from-scratch educational philosophy of the coding-adventures stack.

## How It Fits in the Stack

```
logic_gates → arithmetic → trig → wave
```

The `trig` package provides `sin_approx` and `cos_approx` computed via
Maclaurin series. The `wave` package uses these to generate sampled waveforms.

## Waveforms

| Function        | Description                                        |
|-----------------|--------------------------------------------------- |
| `sine_wave`     | Pure sinusoidal oscillation; no harmonics          |
| `cosine_wave`   | Sine shifted 90°; starts at peak amplitude         |
| `square_wave`   | Alternates ±amplitude; rich in odd harmonics       |
| `sawtooth_wave` | Linear ramp with reset; contains all harmonics     |
| `triangle_wave` | Linear rise/fall; odd harmonics, softer than square|
| `dc_offset`     | Constant-value signal                              |

## Mixing Utilities

| Function      | Description                              |
|---------------|------------------------------------------|
| `add_waves`   | Element-wise sum of exactly two waves    |
| `scale_wave`  | Multiply every sample by a scalar        |
| `mix_waves`   | Element-wise sum of N waves              |

## Usage

```lua
local wave = require("coding_adventures.wave")

-- 440 Hz sine wave (concert A), 1 second at CD quality
local samples = wave.sine_wave(440, 1.0, 0, 44100, 44100)

-- Mix two harmonics
local fundamental = wave.sine_wave(440, 0.8, 0, 44100, 44100)
local harmonic    = wave.sine_wave(880, 0.2, 0, 44100, 44100)
local combined    = wave.add_waves(fundamental, harmonic)

-- Mix three waves at once
local mix = wave.mix_waves({
    wave.sine_wave(440,  1.0, 0, 44100, 44100),
    wave.sine_wave(880,  0.5, 0, 44100, 44100),
    wave.sine_wave(1320, 0.25, 0, 44100, 44100),
})

-- Add DC bias and halve amplitude
local biased = wave.mix_waves({
    wave.scale_wave(samples, 0.5),
    wave.dc_offset(0.1, 44100),
})
```

## Function Reference

### `sine_wave(frequency, amplitude, phase, sample_rate, num_samples)`

Generates `num_samples` samples of:

    y(i) = amplitude * sin(2π * frequency * (i/sample_rate) + phase)

### `cosine_wave(frequency, amplitude, phase, sample_rate, num_samples)`

Same as `sine_wave` but uses cosine. Equivalent to a sine with phase +π/2.

### `square_wave(frequency, amplitude, sample_rate, num_samples)`

Hard-clips a sine to ±amplitude. Every sample is exactly `+amplitude` or
`-amplitude`. 50% duty cycle.

### `sawtooth_wave(frequency, amplitude, sample_rate, num_samples)`

Linear ramp using:

    phase_frac = t*f - floor(t*f + 0.5)
    y = 2 * amplitude * phase_frac

### `triangle_wave(frequency, amplitude, sample_rate, num_samples)`

Linear rise and fall derived from a double-frequency sawtooth via absolute value.

### `dc_offset(value, num_samples)`

Returns an array of `num_samples` all equal to `value`.

### `add_waves(wave1, wave2)`

Element-wise sum. Raises an error if the arrays have different lengths.

### `scale_wave(wave, scalar)`

Returns a new array with every element multiplied by `scalar`.

### `mix_waves(waves)`

Element-wise sum of all arrays in `waves`. All arrays must be the same length.

## Constants

- `wave.TWO_PI` — 6.283185307179586 (re-exported from trig)

## Version

0.1.0

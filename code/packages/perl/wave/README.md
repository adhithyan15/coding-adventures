# CodingAdventures::Wave (Perl)

Signal and waveform generation — pure-Perl library for generating digital
representations of periodic waveforms.

## Overview

This module generates arrays of floating-point samples representing classic
waveforms used in audio synthesis and digital signal processing. It depends on
`CodingAdventures::Trig` for sin/cos computed from first principles via
Maclaurin series.

## How It Fits in the Stack

```
logic_gates → arithmetic → trig → wave
```

## Waveforms

| Function        | Description                                   |
|-----------------|-----------------------------------------------|
| `sine_wave`     | Pure sinusoidal oscillation (no harmonics)    |
| `cosine_wave`   | Sine shifted 90°; starts at peak amplitude    |
| `square_wave`   | Alternates ±amplitude; rich in odd harmonics  |
| `sawtooth_wave` | Linear ramp with reset; contains all harmonics|
| `triangle_wave` | Linear rise/fall; odd harmonics, mellow sound |
| `dc_offset`     | Constant-value signal                         |

## Mixing Utilities

| Function      | Description                              |
|---------------|------------------------------------------|
| `add_waves`   | Element-wise sum of two waves            |
| `scale_wave`  | Multiply every sample by a scalar        |
| `mix_waves`   | Element-wise sum of N waves              |

## Usage

```perl
use CodingAdventures::Wave qw(
    sine_wave cosine_wave square_wave sawtooth_wave triangle_wave
    dc_offset add_waves scale_wave mix_waves
);

# 440 Hz sine wave, 1 second at CD quality
my @samples = sine_wave(440, 1.0, 0, 44100, 44100);

# Mix two harmonics
my @mix = add_waves(
    [ sine_wave(440, 0.8, 0, 44100, 44100) ],
    [ sine_wave(880, 0.2, 0, 44100, 44100) ],
);

# Mix three waves at once
my @rich = mix_waves([
    [ sine_wave(440,  1.0, 0, 44100, 44100) ],
    [ sine_wave(880,  0.5, 0, 44100, 44100) ],
    [ sine_wave(1320, 0.25, 0, 44100, 44100) ],
]);

# Scale and add DC bias
my @biased = mix_waves([
    [ scale_wave(\@samples, 0.5) ],
    [ dc_offset(0.1, 44100) ],
]);
```

## Function Reference

### `sine_wave($freq, $amp, $phase, $sr, $n)`

Returns `$n` samples of `$amp * sin(2π*$freq*t + $phase)`.

### `cosine_wave($freq, $amp, $phase, $sr, $n)`

Returns `$n` samples of `$amp * cos(2π*$freq*t + $phase)`.

### `square_wave($freq, $amp, $sr, $n)`

Returns `$n` samples alternating between `±$amp`.

### `sawtooth_wave($freq, $amp, $sr, $n)`

Linear ramp: `2*$amp*(t*f - floor(t*f + 0.5))`.

### `triangle_wave($freq, $amp, $sr, $n)`

Derived from double-frequency sawtooth via absolute value.

### `dc_offset($val, $n)`

Returns `($val) x $n`.

### `add_waves(\@w1, \@w2)`

Element-wise sum. Dies if lengths differ.

### `scale_wave(\@wave, $scalar)`

Returns each element multiplied by `$scalar`.

### `mix_waves([\@w1, \@w2, ...])`

Element-wise sum of all input arrays.

## Constants

- `$CodingAdventures::Wave::TWO_PI` — 6.283185307179586

## Version

0.01

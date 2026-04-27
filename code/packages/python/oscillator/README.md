# Oscillator

`coding-adventures-oscillator` is the first virtual implementation of
`OSC00: Oscillator and Sampler`.

It models an oscillator as a pure mathematical signal:

```text
time in seconds -> signal value
```

Then it models a sampler as the layer that turns that smooth virtual signal
into ordinary numbers:

```text
continuous signal -> uniform sampler -> sample buffer
```

This is the next rung after `note-frequency`. `note-frequency` answers
"what frequency is `A4`?" This package answers "given a frequency, what value
does the signal have at each time?"

## Why This Is Virtual

This package does not talk to speakers, sound cards, Arduino pins, timers, DACs,
or radios. It is intentionally pure and deterministic. That makes it good for
learning and testing:

- the same oscillator at the same time always returns the same value
- the same sampler always produces the same sample buffer
- no method sleeps or reads wall-clock time

Later packages can put real outputs underneath the same ideas.

## Usage

```python
from oscillator import SineOscillator, UniformSampler

tone = SineOscillator(frequency_hz=440.0)
sampler = UniformSampler(sample_rate_hz=44_100.0)

buffer = sampler.sample(tone, duration_seconds=0.01)

print(buffer.sample_count())          # 441
print(buffer.sample_period_seconds()) # 1 / 44100
print(buffer.samples[:5])             # first five floating-point samples
```

## Sine Oscillator

```python
from oscillator import SineOscillator

wave = SineOscillator(frequency_hz=1.0)

wave.value_at(0.00) # 0.0
wave.value_at(0.25) # 1.0
wave.value_at(0.50) # 0.0
wave.value_at(0.75) # -1.0
```

The formula is:

```text
offset + amplitude * sin(2 * pi * (frequency_hz * time_seconds + phase_cycles))
```

## Square Oscillator

```python
from oscillator import SquareOscillator

clock_line = SquareOscillator(
    frequency_hz=2.0,
    low=0.0,
    high=1.0,
    duty_cycle=0.5,
)

clock_line.value_at(0.000) # 1.0
clock_line.value_at(0.250) # 0.0
clock_line.value_at(0.500) # 1.0
```

This is the virtual signal shape that a clock package can use internally. Clock
consumers should still consume clock edges; they should not have to manually
sample square waves.

## Sampling

A uniform sampler chooses evenly spaced times:

```text
t_n = start_time_seconds + n / sample_rate_hz
```

At 4 Hz for one second, the sample times are:

```text
0.00, 0.25, 0.50, 0.75
```

The endpoint `1.00` is not included because the sampler uses the half-open
interval `[start, end)`.

## Aliasing

The sampler exposes the Nyquist frequency:

```python
from oscillator import nyquist_frequency

nyquist_frequency(44_100.0) # 22050.0
```

This package only reports the limit. It does not implement anti-aliasing
filters yet.

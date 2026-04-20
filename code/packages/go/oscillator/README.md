# Oscillator

`oscillator` is the Go implementation of `OSC00: Oscillator and Sampler`.

It models a signal as a virtual continuous function:

```text
time in seconds -> signal value
```

Then it models a sampler as the layer that turns that smooth virtual signal
into ordinary numbers:

```text
continuous signal -> uniform sampler -> sample buffer
```

This is intentionally not a sound-card, Arduino, DAC, radio, or real-time timer
package. It is the deterministic math layer that those later packages can build
on.

## Usage

```go
package main

import (
	"fmt"

	oscillator "github.com/adhithyan15/coding-adventures/code/packages/go/oscillator"
)

func main() {
	tone, err := oscillator.NewSineOscillator(440.0)
	if err != nil {
		panic(err)
	}

	sampler, err := oscillator.NewUniformSampler(44100.0)
	if err != nil {
		panic(err)
	}

	buffer, err := sampler.Sample(tone, 0.01)
	if err != nil {
		panic(err)
	}

	fmt.Println(buffer.SampleCount())
	fmt.Println(buffer.SamplePeriodSeconds())
	fmt.Println(buffer.Samples[:5])
}
```

## Sine Oscillator

```go
wave, _ := oscillator.NewSineOscillator(1.0)

wave.ValueAt(0.00) // 0.0
wave.ValueAt(0.25) // 1.0
wave.ValueAt(0.50) // 0.0
wave.ValueAt(0.75) // -1.0
```

The formula is:

```text
offset + amplitude * sin(2 * pi * (frequency_hz * time_seconds + phase_cycles))
```

## Square Oscillator

```go
clockLine, _ := oscillator.NewSquareOscillatorWithOptions(
	2.0, // frequency_hz
	0.0, // low
	1.0, // high
	0.5, // duty_cycle
	0.0, // phase_cycles
)

clockLine.ValueAt(0.000) // 1.0
clockLine.ValueAt(0.250) // 0.0
clockLine.ValueAt(0.500) // 1.0
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

```go
nyquist, _ := oscillator.NyquistFrequency(44100.0)
// nyquist == 22050.0
```

This package only reports the limit. It does not implement anti-aliasing
filters yet.

# OSC00: Oscillator and Sampler

## 1. Overview

An oscillator is anything that repeats over time.

That sounds simple, but this one idea shows up everywhere in the repo:

- an audio tone such as `A4 = 440 Hz`
- a CPU clock toggling billions of times per second
- a radio carrier wave used for remote-control communication
- a pulse-width modulation signal driving a motor or LED
- a test signal used to debug filters, samplers, and waveform code

This spec defines two deliberately separate abstractions:

- a **continuous oscillator**, which is a virtual mathematical function
- a **sampler**, which chooses concrete times and asks the oscillator for values

The split matters. The oscillator answers:

> "If I could look at this signal at any exact time, what value would I see?"

The sampler answers:

> "If I only measure this signal at regular intervals, what list of values do I get?"

This lets us teach the same concept across music, electronics, digital logic,
and radio without committing to a real sound card, ADC, DAC, or operating system
timer yet.

## 2. Relationship to Existing Specs

This layer sits between the repo's current waveform specs:

- `PHY03: Analog Waveform` defines the broad idea of a continuous-time signal.
- `PHY04: Discrete Waveform` defines sampled data with a sample rate.
- `MUS00: Note-to-Frequency Mapping` converts names like `A4` into Hertz.

This spec gives us the missing bridge:

```text
typed note  ->  frequency  ->  oscillator  ->  sampler  ->  samples
   A4             440 Hz        x(t)           x[n]         audio buffer
```

The same bridge works for a CPU clock:

```text
frequency  ->  square oscillator  ->  edge detector  ->  clock ticks
 1 GHz         high/low over time      rising edges       CPU steps
```

and for radio communication:

```text
message  ->  carrier oscillator  ->  modulation  ->  sampled output
 bits        sine wave in Hz         AM/FM/etc.      transmit buffer
```

## 3. Mental Model

Imagine drawing a smooth sine wave on transparent glass. The drawing exists at
every possible point along the horizontal time axis, even between the tick marks.
That drawing is the continuous oscillator.

Now imagine placing graph paper behind the glass and only writing down the wave
height at each grid line. Those written-down numbers are samples.

The graph-paper spacing is the sample period:

$$sample\_period = \frac{1}{sample\_rate}$$

A higher sample rate means the grid lines are closer together, so we keep more
detail. A lower sample rate means the grid lines are farther apart, so we may
miss important motion between measurements.

## 4. Core Time Model

All oscillator and sampler APIs use seconds as the base unit of time.

Required time rules:

- `time_seconds` is a finite real number represented by the host language's
  normal floating-point type.
- `0.0` means the signal's local starting point, not necessarily wall-clock time.
- negative times are allowed for pure continuous evaluation unless a specific
  implementation has a documented reason to reject them.
- samplers operate over half-open intervals: `[start_time, end_time)`.

The half-open rule means a one-second sample at 4 Hz produces times:

```text
0.00, 0.25, 0.50, 0.75
```

It does **not** include `1.00`, because `1.00` is the first instant after the
one-second window.

## 5. Core Signal Model

A continuous signal is a pure function of time:

$$x(t) \rightarrow value$$

The minimum shared interface is:

```text
ContinuousSignal.value_at(time_seconds) -> float
```

The name may be adapted to each language's conventions:

- `value_at(t)` in Python and Ruby
- `ValueAt(t)` in Go
- `valueAt(t)` in TypeScript, Java, Kotlin, Dart, and C#
- `value_at` or equivalent in Rust
- `valueAt` or an idiomatic function in functional languages

The important contract is semantic, not spelling:

- the caller supplies a time in seconds
- the signal returns the value at that time
- the method does not sleep, wait, read the real clock, or mutate hidden state

## 6. Oscillator Parameters

An oscillator is a continuous signal whose value repeats.

The common parameters are:

| Parameter | Meaning | Unit |
|-----------|---------|------|
| `frequency_hz` | cycles per second | Hertz |
| `amplitude` | distance from center to peak | signal units |
| `phase_cycles` | where in the cycle the oscillator starts | cycles |
| `offset` | center value around which the signal moves | signal units |

This spec uses `phase_cycles` instead of radians for the public API because it
is easier to explain across music, clocks, and radio:

- `0.0` cycles means start at the beginning of the cycle
- `0.25` cycles means start one quarter-cycle later
- `0.5` cycles means start halfway through the cycle
- `1.0` cycles means the same as `0.0`

Implementations may convert cycles to radians internally:

$$radians = 2\pi \cdot phase\_cycles$$

## 7. Sine Oscillator

A sine oscillator is the smoothest basic oscillator and is the first shape we
need for audio tones and radio carriers.

Formula:

$$x(t) = offset + amplitude \cdot \sin(2\pi(f \cdot t + phase\_cycles))$$

Required constructor shape:

```text
SineOscillator(
    frequency_hz,
    amplitude = 1.0,
    phase_cycles = 0.0,
    offset = 0.0
)
```

Required behavior:

- `frequency_hz` controls how many complete cycles happen per second.
- `amplitude` controls the peak distance away from `offset`.
- `phase_cycles` shifts where the oscillator begins.
- `offset` moves the center line up or down.

For a default 1 Hz sine oscillator:

| Time | Value |
|-----:|------:|
| `0.00` | `0.0` |
| `0.25` | `1.0` |
| `0.50` | `0.0` |
| `0.75` | `-1.0` |
| `1.00` | `0.0` |

Normal floating-point tolerance applies to values that are mathematically zero.
For example, a host language may return `1.224646799e-16` instead of exactly
`0.0` at half a cycle.

## 8. Square Oscillator

A square oscillator switches between two levels. It is the basic shape for
digital clocks, pulse trains, and many beginner audio experiments.

Required constructor shape:

```text
SquareOscillator(
    frequency_hz,
    low = -1.0,
    high = 1.0,
    duty_cycle = 0.5,
    phase_cycles = 0.0
)
```

Definitions:

- `low` is the value during the low part of the cycle.
- `high` is the value during the high part of the cycle.
- `duty_cycle` is the fraction of the cycle spent high.
- `phase_cycles` shifts where the cycle begins.

The cycle position is:

$$position = fractional\_part(frequency\_hz \cdot t + phase\_cycles)$$

`fractional_part(x)` must always return a value in `[0.0, 1.0)`, even when
`x` is negative. One portable definition is:

```text
fractional_part(x) = x - floor(x)
```

Then:

```text
if position < duty_cycle:
    value = high
else:
    value = low
```

For a 2 Hz square oscillator with `low = 0`, `high = 1`, and
`duty_cycle = 0.5`:

| Time | Position in cycle | Value |
|-----:|------------------:|------:|
| `0.000` | `0.0` | `1` |
| `0.125` | `0.25` | `1` |
| `0.250` | `0.5` | `0` |
| `0.375` | `0.75` | `0` |
| `0.500` | `0.0` | `1` |

This square oscillator is continuous in the API sense because it can be queried
at any time, but the waveform itself has jumps. That is expected. A discontinuous
shape can still be a continuous-time signal.

## 9. Clock Interpretation

A CPU clock should be built internally from oscillator ideas, but ordinary clock
consumers should not have to care about that internal construction.

At the public API boundary, a clock package should still feel like a clock:

```text
clock.tick() -> ClockEdge
```

or:

```text
clock.edges(duration_seconds) -> ClockEdge stream
```

The consumer should not need to know whether the clock uses a square oscillator,
an event counter, a divider, or a future jitter model internally. Flip-flops,
CPU cores, GPU pipelines, and bus simulators should consume clock edges, not
sample oscillator waveforms by hand.

For learners and debuggers, however, the layers should remain visible:

```text
SquareOscillator
  -> ClockSignal
  -> EdgeDetector
  -> ClockEdge
  -> CPU, GPU, flip-flop, or bus consumer
```

That is the intended shape of the abstraction: hide the machinery for normal
use, but make it inspectable for people who want to peek behind the curtain.

A square oscillator answers:

```text
at time t, the clock line is 0 or 1
```

Sequential digital logic usually wants a different question:

```text
did a rising or falling edge happen?
```

That edge detection belongs in a clock or logic package, not in the base
oscillator itself. The base layer should only define the continuous high/low
signal. A higher layer may turn transitions into records such as:

```text
ClockEdge(
    time_seconds,
    cycle_index,
    previous_value,
    current_value,
    is_rising,
    is_falling
)
```

This keeps the oscillator general enough for audio and radio while still making
it useful as the mathematical foundation beneath digital clocks.

## 10. Sampler

A sampler turns a continuous signal into discrete values by choosing times.

The minimum shared interface is:

```text
Sampler.sample(signal, duration_seconds, start_time_seconds = 0.0) -> SampleBuffer
```

A uniform sampler is configured by:

```text
UniformSampler(sample_rate_hz)
```

The sample period is:

$$sample\_period = \frac{1}{sample\_rate\_hz}$$

Sample index `n` maps to time:

$$t_n = start\_time\_seconds + \frac{n}{sample\_rate\_hz}$$

For `sample_rate_hz = 4`, `duration_seconds = 1`, and
`start_time_seconds = 0`, the sampler evaluates the signal at:

| Sample index | Time |
|-------------:|-----:|
| `0` | `0.00` |
| `1` | `0.25` |
| `2` | `0.50` |
| `3` | `0.75` |

## 11. Sample Count Rules

The default sample count for a duration is:

$$sample\_count = floor(duration\_seconds \cdot sample\_rate\_hz)$$

That formula is mathematical. Real implementations use floating-point numbers,
so they should avoid accidental off-by-one results when a product lands extremely
close to an integer. For example, a host language should not produce `479`
samples for a duration and sample rate that mathematically multiply to `480`
just because the intermediate value was represented as `479.99999999999994`.

Examples:

| Duration | Sample rate | Sample count |
|---------:|------------:|-------------:|
| `1.0` second | `44100` Hz | `44100` |
| `0.5` seconds | `48000` Hz | `24000` |
| `0.01` seconds | `48000` Hz | `480` |
| `0.0` seconds | `44100` Hz | `0` |

Implementations may also expose an explicit-count API:

```text
Sampler.sample_count(signal, sample_count, start_time_seconds = 0.0)
```

That variant is useful when the caller already knows the exact number of samples
needed, such as a fixed-size audio block.

## 12. Sample Buffer

A `SampleBuffer` stores sampled values and enough timing metadata to interpret
them.

Required fields:

| Field | Meaning |
|-------|---------|
| `samples` | ordered list of floating-point values |
| `sample_rate_hz` | samples per second |
| `start_time_seconds` | time of sample index `0` |

Derived values:

| Method | Formula |
|--------|---------|
| `sample_count()` | `len(samples)` |
| `sample_period_seconds()` | `1 / sample_rate_hz` |
| `duration_seconds()` | `sample_count / sample_rate_hz` |
| `time_at(index)` | `start_time_seconds + index / sample_rate_hz` |

The buffer stores floating-point values. Conversion to PCM integers, WAV files,
speaker APIs, or DAC codes belongs in later packages.

## 13. Streaming

Some sampled outputs are small enough to hold in memory. Others are not.

For that reason, implementations should prefer exposing both forms when the
host language makes it natural:

- `sample(...) -> SampleBuffer` for small educational examples
- `samples(...) -> Iterator<float>` or stream for long-running signals

The streaming form must produce the same values as the buffer form for the same
configuration.

## 14. Aliasing and Nyquist Limit

Sampling can lie to us.

If a signal wiggles too fast relative to the sample rate, the sampled values can
look like a different lower-frequency signal. This is called aliasing.

For a uniform sampler, the Nyquist frequency is:

$$nyquist\_frequency = \frac{sample\_rate\_hz}{2}$$

The sampler should expose:

```text
nyquist_frequency() -> float
```

This first oscillator package does not need to implement anti-aliasing filters.
It should, however, document that clean sampling requires signal content below
the Nyquist frequency. Later DSP packages can add filtering and resampling.

Example:

- a 44,100 Hz audio sampler has a Nyquist frequency of 22,050 Hz
- a 440 Hz `A4` tone is safely below that limit
- a 30,000 Hz sine sampled at 44,100 Hz will alias

## 15. Validation Rules

Every implementation should reject invalid configuration early and explicitly.

Required validation:

- `frequency_hz` must be finite and greater than or equal to `0.0`.
- `amplitude` must be finite and greater than or equal to `0.0`.
- `phase_cycles` must be finite.
- `offset`, `low`, and `high` must be finite.
- `duty_cycle` must be finite and satisfy `0.0 < duty_cycle < 1.0`.
- `sample_rate_hz` must be finite and greater than `0.0`.
- `duration_seconds` must be finite and greater than or equal to `0.0`.
- explicit `sample_count` must be an integer greater than or equal to `0`.

For `frequency_hz = 0.0`:

- a sine oscillator returns a constant value based on phase, amplitude, and
  offset
- a square oscillator returns the value implied by its initial phase and duty
  cycle
- edge-producing clock layers should treat a zero-frequency source as having no
  edges

Implementations must not silently return `NaN` for invalid input.

## 16. Determinism

This package is a simulation primitive, not a real-time scheduler.

Required deterministic behavior:

- the same oscillator configuration and time must produce the same value
- sampling the same signal with the same sampler must produce the same buffer
- no method may depend on wall-clock time
- no method may sleep
- no method may use global mutable state

Stateful real-time playback belongs in later audio-device or runtime packages.
This layer is intentionally pure so that tests, tutorials, and simulations can
be exact and repeatable.

## 17. Numerical Tolerance

Implementations will use floating-point arithmetic.

Tests should compare floating-point values with tolerance rather than exact
equality unless the value is structurally exact, such as a square oscillator's
`low` or `high` output.

Recommended tolerance for cross-language parity tests:

```text
absolute_tolerance = 1e-9
```

For very large times or very high frequencies, floating-point phase reduction
can lose precision. That is acceptable for this introductory package. A future
high-precision oscillator can define stricter behavior if needed.

## 18. Cross-Language Parity Vectors

Every implementation should agree on these examples within normal
double-precision tolerance.

### 18.1 Sine Oscillator

Configuration:

```text
SineOscillator(frequency_hz = 1.0)
```

Expected values:

| Time | Expected value |
|-----:|---------------:|
| `0.00` | `0.0` |
| `0.25` | `1.0` |
| `0.50` | `0.0` |
| `0.75` | `-1.0` |
| `1.00` | `0.0` |

Configuration:

```text
SineOscillator(frequency_hz = 1.0, amplitude = 2.0, offset = 3.0)
```

Expected values:

| Time | Expected value |
|-----:|---------------:|
| `0.00` | `3.0` |
| `0.25` | `5.0` |
| `0.75` | `1.0` |

Configuration:

```text
SineOscillator(frequency_hz = 1.0, phase_cycles = 0.25)
```

Expected values:

| Time | Expected value |
|-----:|---------------:|
| `0.00` | `1.0` |
| `0.25` | `0.0` |

### 18.2 Square Oscillator

Configuration:

```text
SquareOscillator(
    frequency_hz = 2.0,
    low = 0.0,
    high = 1.0,
    duty_cycle = 0.5
)
```

Expected values:

| Time | Expected value |
|-----:|---------------:|
| `0.000` | `1.0` |
| `0.125` | `1.0` |
| `0.250` | `0.0` |
| `0.375` | `0.0` |
| `0.500` | `1.0` |

### 18.3 Uniform Sampler

Configuration:

```text
signal = SineOscillator(frequency_hz = 1.0)
sampler = UniformSampler(sample_rate_hz = 4.0)
buffer = sampler.sample(signal, duration_seconds = 1.0)
```

Expected sample times:

```text
0.00, 0.25, 0.50, 0.75
```

Expected sample values:

```text
0.0, 1.0, 0.0, -1.0
```

Expected metadata:

| Property | Expected value |
|----------|---------------:|
| `sample_count()` | `4` |
| `sample_period_seconds()` | `0.25` |
| `duration_seconds()` | `1.0` |
| `nyquist_frequency()` | `2.0` |

## 19. Rollout Scope

The first implementation package should stay small and educational.

Required v1 types:

- `ContinuousSignal`
- `SineOscillator`
- `SquareOscillator`
- `UniformSampler`
- `SampleBuffer`

Required v1 helpers:

- `sample_count_for_duration(duration_seconds, sample_rate_hz)`
- `time_at_sample(index, sample_rate_hz, start_time_seconds = 0.0)`
- `nyquist_frequency(sample_rate_hz)`

Suggested package name:

```text
oscillator
```

Suggested rollout target:

- start with Python as the teaching prototype
- then port the package across the same runtime languages as `MUS00`
- keep Starlark out of scope unless it later becomes a user-facing runtime
  package language

## 20. Out of Scope for v1

This first layer intentionally does not include:

- real speaker playback
- sound-card APIs
- WAV or MP3 encoding
- ADC or DAC hardware modeling
- anti-aliasing filters
- resampling
- envelopes such as attack, decay, sustain, and release
- harmonics, overtones, or instrument timbre
- symbolic calculus or differential-equation physics models
- real-time scheduling
- jitter, drift, phase noise, or imperfect hardware clocks
- modulation schemes such as AM, FM, FSK, or PWM

Those are all good later layers. The point of this spec is to define the clean
mathematical seam that they can build on.

## 21. Why the Sampler Sits on Top

The oscillator should remain a virtual continuous function because that mirrors
the thing we are trying to model: a signal that exists over time.

The sampler should sit on top because sampling is one possible way to observe
that signal. Audio rendering samples a signal before sending it to a speaker.
Digital simulation may sample a square wave to find edges. Radio experiments may
sample a carrier before modulation or transmission.

Keeping the layers separate gives us a reusable ladder:

```text
frequency
  -> continuous oscillator
  -> sampler
  -> sample buffer
  -> encoder, speaker, DAC, edge detector, or radio model
```

That ladder is the path from "type `A4`" to "hear a tone", but it is also the
path from "create a 1 MHz clock" to "step a simulated CPU".

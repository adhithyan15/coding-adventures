# MUS01: Note-to-Sound Signal Chain

## 1. Overview

This spec defines the first complete educational path from a typed musical note
to something that can be heard.

The goal is not to jump straight from `"A4"` to "play a sound". The goal is to
make every box in the chain visible:

```text
typed note
  -> frequency
  -> continuous oscillator
  -> sampler
  -> digital sample values
  -> PCM integers
  -> virtual DAC
  -> virtual speaker
  -> playback sink
```

For the first implementation, every layer may remain virtual and deterministic.
That means:

- no real sound card is required
- no operating-system audio callback is required
- no Arduino or microcontroller is required
- tests can inspect every intermediate representation

Later packages can replace the final virtual playback sink with a real OS audio
device, an Arduino PWM output, an I2S DAC, or another hardware target.

## 2. Relationship to Existing Specs

This spec composes the layers already introduced by:

- `MUS00: Note-to-Frequency Mapping`
- `OSC00: Oscillator and Sampler`
- `PHY03: Analog Waveform`
- `PHY04: Discrete Waveform`

`MUS00` answers:

> "When the user types `A4`, what frequency do we mean?"

`OSC00` answers:

> "Given a frequency and a time, what is the wave value?"

This spec answers:

> "How do those wave values become the digital numbers, analog voltages, and
> speaker motion that eventually make sound?"

The first end-to-end path is:

```text
"A4"
  -> 440.0 Hz
  -> sine wave x(t) = sin(2*pi*440*t)
  -> floating-point samples at 44,100 samples/second
  -> signed 16-bit PCM samples
  -> virtual DAC voltage over time
  -> virtual speaker pressure over time
```

## 3. Beginner Mental Model

Imagine a keyboard key labeled `A4`.

Pressing that key does not directly create a file or a stream of numbers.
Conceptually, it starts a chain:

1. The note name tells us the frequency.
2. The frequency tells us how fast the wave wiggles.
3. The oscillator defines the smooth ideal wave.
4. The sampler takes snapshots of that wave.
5. The digital encoder stores each snapshot as a finite integer.
6. The DAC turns each integer into a voltage over time.
7. The speaker turns changing voltage into moving air.

The important thing is that each layer changes the shape of the information:

| Layer | Question | Example answer |
|-------|----------|----------------|
| Note parser | What did the human type? | `A4` |
| Frequency mapper | How many cycles per second? | `440.0 Hz` |
| Oscillator | What is the ideal value at time `t`? | `sin(2*pi*440*t)` |
| Sampler | What values did we measure? | `0.0, 0.0626, ...` |
| PCM encoder | What integers can a machine store? | `0, 2053, ...` |
| DAC | What voltage does that integer mean? | `0.0 V, 0.063 V, ...` |
| Speaker | What air-pressure proxy results? | `0.0, 0.063, ...` |
| Playback | Where do the samples go? | file, virtual trace, or device |

## 4. Core End-to-End Types

The exact class, struct, or function names may vary by language, but every
implementation should expose the same conceptual values.

### 4.1 `NoteEvent`

A note event combines a typed note with timing and loudness:

```text
NoteEvent(
    note,
    duration_seconds,
    amplitude = 0.8,
    start_time_seconds = 0.0
)
```

Fields:

| Field | Meaning |
|-------|---------|
| `note` | a note string such as `A4`, or a parsed `Note` from `MUS00` |
| `duration_seconds` | how long the note lasts |
| `amplitude` | normalized loudness before PCM conversion |
| `start_time_seconds` | where the note begins in the rendered timeline |

The first version only needs single-note rendering. The type still includes
`start_time_seconds` because melodies, rests, and overlapping notes need a
timeline later.

### 4.2 `RenderedNote`

A rendered note is a teaching object that keeps the intermediate boxes:

```text
RenderedNote(
    note,
    frequency_hz,
    oscillator,
    floating_samples,
    pcm_samples,
    dac_signal,
    speaker_signal
)
```

An implementation may store references, copies, or lightweight summaries, but
tests and examples must be able to inspect:

- the parsed note
- the computed frequency
- the oscillator parameters
- the sampled floating-point values
- the quantized PCM values
- the DAC mapping from integer to voltage
- the speaker model output

## 5. Stage 1: Note to Frequency

The first stage delegates to `MUS00`.

Input:

```text
"A4"
```

Output:

```text
440.0 Hz
```

Required behavior:

- note parsing must follow `MUS00`
- tuning must default to `A4 = 440 Hz`
- frequency is measured in Hertz, meaning cycles per second
- malformed notes must fail before any oscillator or sampler is built

Worked example:

```text
note = "A4"
semitones_from_a4 = 0
frequency_hz = 440 * 2^(0/12)
frequency_hz = 440.0
```

## 6. Stage 2: Frequency to Oscillator

The second stage delegates to `OSC00`.

For the first audio tone, the oscillator is a sine wave:

```text
SineOscillator(
    frequency_hz = note_frequency,
    amplitude = note_event.amplitude,
    phase_cycles = 0.0,
    offset = 0.0
)
```

For `A4` at amplitude `0.8`:

```text
x(t) = 0.8 * sin(2*pi*440*t)
```

Required behavior:

- the oscillator is still a continuous virtual function
- it does not know about PCM, speakers, files, or devices
- it can be queried at any finite time in seconds
- the first implementation should use a sine oscillator only

Out of scope for this version:

- envelopes such as attack, decay, sustain, release
- harmonics or instrument timbre
- chords
- multiple simultaneous oscillators
- sample-accurate phase continuity between repeated notes

Those are important, but they are later layers.

## 7. Stage 3: Oscillator to Sampler

The sampler chooses a finite grid of times and evaluates the oscillator.

For a uniform sampler:

```text
t_n = start_time_seconds + n / sample_rate_hz
```

For a one-second note at 44,100 Hz:

```text
sample_count = floor(1.0 * 44100) = 44100
```

The sampled values are floating-point numbers, usually in the normalized audio
range `[-1.0, 1.0]`.

Example for a 1 Hz sine wave sampled at 4 Hz:

| Sample index | Time | Floating sample |
|-------------:|-----:|----------------:|
| `0` | `0.00` | `0.0` |
| `1` | `0.25` | `1.0` |
| `2` | `0.50` | `0.0` |
| `3` | `0.75` | `-1.0` |

Required behavior:

- sample intervals are half-open: `[start, end)`
- sample rates must be finite and greater than zero
- durations must be finite and greater than or equal to zero
- the sampler must expose the sample rate and sample count
- the sampler must not talk to a device or sleep in real time

## 8. Stage 4: Floating Samples to Digital PCM

Floating samples are convenient for math, but most audio hardware and audio
files use fixed-size integer samples.

The first digital representation is signed PCM:

```text
PCMFormat(
    sample_rate_hz = 44100,
    channel_count = 1,
    bit_depth = 16,
    full_scale_voltage = 1.0
)
```

For signed 16-bit PCM:

```text
minimum integer = -32768
maximum integer =  32767
```

The normalized floating range maps to integer range:

```text
-1.0 -> -32768
 0.0 ->      0
 1.0 ->  32767
```

Because the negative side has one extra integer in two's-complement encoding,
the portable mapping is:

```text
if sample >= 0:
    pcm = round(sample * 32767)
else:
    pcm = round(sample * 32768)
```

Then clamp to `[-32768, 32767]`.

### 8.1 Why Clipping Exists

A floating sample outside `[-1.0, 1.0]` cannot fit into normalized PCM without
distortion.

The first implementation should use explicit clipping:

```text
clipped = min(1.0, max(-1.0, sample))
```

This mirrors real audio systems: if the signal is too loud for the representable
range, the peaks flatten. That flattened waveform is called clipping.

For teaching, the encoder should also expose whether clipping occurred:

```text
PCMBuffer.clipped_sample_count
```

Required behavior:

- `NaN` and infinite samples are rejected
- finite samples outside `[-1.0, 1.0]` are clipped, not rejected
- clipping count is observable
- signed 16-bit little-endian PCM is the first required format
- mono audio is the first required channel layout

Out of scope for this version:

- stereo panning
- dithering
- floating-point WAV files
- compressed formats such as MP3, AAC, Ogg Vorbis, or Opus
- resampling between sample rates

## 9. Stage 5: PCM to Virtual DAC

A Digital-to-Analog Converter, or DAC, turns digital numbers into a physical
voltage.

In this spec, the DAC is still virtual. It does not touch real hardware. It
answers:

> "If this PCM sample reached an idealized DAC, what voltage would it output?"

The first virtual DAC model is zero-order hold.

That means each PCM sample becomes a constant voltage that lasts until the next
sample time:

```text
sample 0 voltage holds from t0 to t1
sample 1 voltage holds from t1 to t2
sample 2 voltage holds from t2 to t3
...
```

For signed 16-bit PCM with `full_scale_voltage = 1.0`:

```text
if pcm >= 0:
    voltage = pcm / 32767 * full_scale_voltage
else:
    voltage = pcm / 32768 * full_scale_voltage
```

Examples:

| PCM integer | Voltage with `full_scale_voltage = 1.0` |
|------------:|----------------------------------------:|
| `-32768` | `-1.0 V` |
| `0` | `0.0 V` |
| `32767` | `1.0 V` |

The resulting DAC output is a continuous-time signal in the API sense:

```text
dac_signal.value_at(time_seconds) -> voltage
```

But it is not smooth. It is a staircase:

```text
      ┌─────┐
      │     │
──────┘     └──────
```

Real DACs add reconstruction filters, output impedance, noise, and bandwidth
limits. The first virtual DAC intentionally does not. The staircase is easier
to inspect and test.

Required behavior:

- DAC input is a PCM buffer plus format metadata
- DAC output is a virtual analog signal
- zero-order hold is the first required reconstruction mode
- querying before the first sample or after the buffer ends must be specified

For V1:

```text
value_at(t) before start -> 0.0 volts
value_at(t) after end    -> 0.0 volts
```

## 10. Stage 6: Virtual DAC to Virtual Speaker

A speaker turns changing voltage into physical motion. The motion pushes air,
and changing air pressure is what our ears perceive as sound.

Real speakers are complicated:

- voice coils
- magnets
- cones
- suspension
- enclosure resonance
- frequency response
- nonlinear distortion

This spec deliberately starts with a toy speaker model:

```text
speaker_pressure(t) = speaker_gain * dac_voltage(t)
```

This is not a physics-accurate speaker. It is a visible educational boundary.
It lets us say:

> "Here is where voltage becomes a sound-pressure-like signal."

Required behavior:

- the speaker model consumes a virtual analog signal
- the speaker model exposes another virtual signal
- the first model is linear gain only
- the default `speaker_gain` is `1.0`

Out of scope for this version:

- cone displacement integrals
- acoustic impedance
- room simulation
- frequency-dependent speaker response
- psychoacoustic loudness

Those later models can replace the toy speaker without changing the earlier
note, oscillator, sampler, PCM, or DAC layers.

## 11. Stage 7: Playback Sink

The playback sink is the final consumer.

There are three important kinds of sink:

| Sink | Meaning | V1 status |
|------|---------|-----------|
| virtual sink | stores the rendered chain for inspection | required |
| file sink | writes PCM into a container such as WAV | allowed |
| device sink | sends samples to OS audio or hardware | out of scope |

The first package should prove the virtual path. It may also write a WAV file
because WAV is just a simple container around PCM data, but WAV writing must not
hide the conceptual layers.

The intended teaching API is not:

```text
play("A4")
```

as a magic black box.

It is:

```text
render_note_to_sound_chain("A4", duration_seconds = 1.0)
```

and then:

```text
chain.frequency_hz
chain.floating_samples
chain.pcm_samples
chain.dac_signal.value_at(0.0001)
chain.speaker_signal.value_at(0.0001)
```

A convenience `play` function may come later, but it should be built on top of
the visible chain, not replace it.

## 12. WAV as an Optional Container

WAV is useful because most operating systems can play it without special audio
libraries.

But WAV is not the sound itself. It is a file format that stores the digital
samples and metadata:

- sample rate
- channel count
- bit depth
- PCM byte data

For V1, a WAV writer may support:

```text
write_wav(path, pcm_buffer)
to_wav_bytes(pcm_buffer) -> bytes
```

Required WAV constraints:

- RIFF/WAVE container
- `fmt ` chunk with PCM format code `1`
- `data` chunk containing little-endian signed 16-bit PCM
- mono only
- no compression
- no metadata chunks required

WAV writing is a convenient proof that the digital stage can be handed to real
software, but it is not a replacement for the DAC and speaker simulations.

## 13. Timing and Units

Every layer must use explicit units.

| Quantity | Unit |
|----------|------|
| note frequency | Hertz |
| time | seconds |
| sample rate | samples per second |
| floating sample | normalized amplitude |
| PCM sample | signed integer |
| DAC output | volts |
| speaker output | normalized pressure proxy |

No layer should rely on wall-clock time in V1.

Rendering a one-second note should compute the one-second buffer as fast as the
program can, not wait one real second.

Real-time playback is a later sink.

## 14. Aliasing and Safety Limits

The sampler cannot represent frequencies above the Nyquist limit:

```text
nyquist_frequency = sample_rate_hz / 2
```

The first implementation should reject note frequencies at or above Nyquist:

```text
frequency_hz < sample_rate_hz / 2
```

This is stricter than merely generating a wrong aliased tone, and it gives a
beginner a clean error:

> "This sample rate is too low to represent this note."

Required safety limits:

- `sample_rate_hz` must be finite and greater than zero
- `duration_seconds` must be finite and greater than or equal to zero
- `duration_seconds * sample_rate_hz` must fit in the host language's practical
  memory limits
- implementations must expose a configurable `max_sample_count`
- default `max_sample_count` should be conservative enough for tests and demos

Suggested default:

```text
max_sample_count = 10_000_000
```

At 44,100 Hz mono, that is about 226 seconds of audio before PCM storage.
Implementations may choose a smaller default if their language ecosystem needs
it, but they must document the value.

## 15. Validation Rules

The first implementation must reject:

- malformed note strings
- non-finite frequency, time, duration, amplitude, sample rate, or voltage
- negative duration
- negative sample rate
- zero sample rate
- negative bit depth
- unsupported bit depth
- unsupported channel count
- note frequencies that violate Nyquist
- render requests whose sample count exceeds `max_sample_count`

The first implementation must clip, not reject:

- finite floating samples outside `[-1.0, 1.0]` during PCM encoding

The first implementation must preserve:

- sample order
- sample count
- sample rate metadata
- exact PCM integer values for parity vectors
- deterministic WAV bytes for the same input

## 16. Required V1 Defaults

Every first implementation should use these defaults unless the caller opts out:

| Setting | Default |
|---------|--------:|
| oscillator shape | sine |
| sample rate | `44100.0` Hz |
| duration | caller-specified |
| amplitude | `0.8` |
| phase | `0.0` cycles |
| PCM bit depth | `16` |
| channel count | `1` |
| full-scale voltage | `1.0` V |
| DAC reconstruction | zero-order hold |
| speaker gain | `1.0` |

The default amplitude is `0.8`, not `1.0`, to leave some headroom and avoid
teaching accidental clipping as the normal case.

## 17. Worked Example: `A4` for One Second

Input:

```text
note = "A4"
duration_seconds = 1.0
sample_rate_hz = 44100.0
amplitude = 0.8
```

Stage 1:

```text
frequency_hz = 440.0
```

Stage 2:

```text
oscillator = SineOscillator(
    frequency_hz = 440.0,
    amplitude = 0.8,
    phase_cycles = 0.0,
    offset = 0.0
)
```

Stage 3:

```text
sample_count = floor(1.0 * 44100.0) = 44100
t_0 = 0 / 44100 = 0.0
t_1 = 1 / 44100 = 0.0000226757...
```

The first floating samples are approximately:

```text
sample[0] = 0.8 * sin(2*pi*440*0/44100) = 0.0
sample[1] = 0.8 * sin(2*pi*440*1/44100) = 0.0501186...
sample[2] = 0.8 * sin(2*pi*440*2/44100) = 0.1000404...
```

Stage 4:

```text
pcm[0] = 0
pcm[1] = round(0.0501186... * 32767) = 1642
pcm[2] = round(0.1000404... * 32767) = 3278
```

Stage 5:

```text
dac_voltage[0] = 0 / 32767 = 0.0 V
dac_voltage[1] = 1642 / 32767 = 0.050111...
dac_voltage[2] = 3278 / 32767 = 0.100040...
```

Stage 6:

```text
speaker_pressure_proxy(t) = 1.0 * dac_voltage(t)
```

Stage 7:

The virtual sink can inspect every stage. A file sink may write the PCM data to
a WAV file. A future device sink may send the PCM stream to a real audio output.

## 18. Small Parity Vector: 1 Hz at 4 Hz

This tiny vector is easier for tests than `A4` because the sine values land on
simple quarter-cycle points.

Input:

```text
frequency_hz = 1.0
duration_seconds = 1.0
sample_rate_hz = 4.0
amplitude = 1.0
bit_depth = 16
full_scale_voltage = 1.0
```

Floating samples:

| Index | Time | Float |
|------:|-----:|------:|
| `0` | `0.00` | `0.0` |
| `1` | `0.25` | `1.0` |
| `2` | `0.50` | `0.0` |
| `3` | `0.75` | `-1.0` |

PCM samples:

| Float | PCM |
|------:|----:|
| `0.0` | `0` |
| `1.0` | `32767` |
| `0.0` | `0` |
| `-1.0` | `-32768` |

DAC voltages:

| PCM | Voltage |
|----:|--------:|
| `0` | `0.0 V` |
| `32767` | `1.0 V` |
| `0` | `0.0 V` |
| `-32768` | `-1.0 V` |

With zero-order hold:

```text
dac.value_at(0.00) -> 0.0
dac.value_at(0.24) -> 0.0
dac.value_at(0.25) -> 1.0
dac.value_at(0.49) -> 1.0
dac.value_at(0.50) -> 0.0
dac.value_at(0.74) -> 0.0
dac.value_at(0.75) -> -1.0
dac.value_at(0.99) -> -1.0
dac.value_at(1.00) -> 0.0
```

The `1.00` result is `0.0` because V1 returns silence after the buffer ends.

## 19. First Package Rollout

The first implementation target should be Python because:

- the repo already has Python `note-frequency`
- the repo already has Python `oscillator`
- Python's standard library can write WAV data without a third-party audio
  dependency
- tests can inspect objects and bytes easily

The first proof was allowed to start as one visible teaching package:

```text
code/packages/python/note-audio/
```

That package should not remain the permanent owner of every stage. Once the
chain is proven, each reusable stage should live in its own package so later
audio, radio, hardware, and visualization packages can reuse the same boxes.

Python V1 stage packages:

| Stage | Package | Responsibility |
|-------|---------|----------------|
| note to frequency | `note-frequency` | parse notes and compute Hertz |
| frequency to oscillator | `oscillator` | expose virtual continuous signals |
| oscillator to samples | `oscillator` | expose uniform sampling and sample buffers |
| floating samples to PCM | `pcm-audio` | encode normalized floats into signed PCM buffers |
| PCM to analog voltage | `virtual-dac` | expose zero-order-hold DAC voltage signals |
| voltage to pressure proxy | `virtual-speaker` | expose simple speaker response models |
| PCM to file container | `wav-sink` | write PCM buffers as deterministic WAV bytes/files |
| teaching composition | `note-audio` | wire the stages together and keep all intermediates visible |

The orchestration package may expose convenience types such as `NoteEvent` and
`RenderedNote`, but it should delegate the stage-specific work to the stage
packages above.

Reusable package responsibility:

- compose `note-frequency` and `oscillator`
- encode signed 16-bit PCM
- expose a virtual zero-order-hold DAC
- expose a linear virtual speaker model
- optionally write WAV bytes/files

After decomposition, those bullets describe the complete system, not one
monolithic package.

No package in this first rollout should:

- access speakers directly
- require PortAudio, CoreAudio, ALSA, PulseAudio, WASAPI, or JACK
- hide the intermediate stages behind a black-box `play()` function
- implement instrument timbre yet

## 20. Future Layers

After V1 proves the chain, later specs can add:

- note sequences and rests
- melody rendering such as "Happy Birthday"
- ADSR envelopes
- harmonics and overtones
- instrument presets
- mixing multiple notes
- stereo output
- anti-aliasing filters
- real-time OS audio playback
- Arduino PWM playback
- I2S DAC output
- more realistic speaker physics

The important design rule remains:

> Convenience APIs may be added, but the layers must stay visible for learners
> who want to peek behind the abstraction.

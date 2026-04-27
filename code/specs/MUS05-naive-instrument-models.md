# MUS05: Naive Instrument Models and Timbre

## 1. Overview

`MUS00` maps note names to frequencies. `MUS01` turns one frequency into a
simple sine tone. `MUS04` introduces named instruments and multiple tracks.

This spec defines the first instrument model:

```text
note frequency + instrument profile -> shaped waveform
```

The goal is not realism yet. The goal is to make a virtual keyboard and virtual
orchestra possible with a beginner-friendly model:

- the same note keeps the same fundamental frequency across instruments
- instruments differ by harmonic content, envelope, gain, and small variation
- every layer remains inspectable and deterministic by default
- later packages can replace the naive model with physical simulations

## 2. Beginner Mental Model

Pressing `A4` on different instruments still means:

```text
fundamental_frequency = 440 Hz
```

But the sound is not only one sine wave. A simple instrument tone is a sum of
related sine waves:

```text
1 * 440 Hz   -> fundamental
2 * 440 Hz   -> second harmonic
3 * 440 Hz   -> third harmonic
4 * 440 Hz   -> fourth harmonic
...
```

The instrument decides how loud each harmonic is.

For example:

```text
flute A4 ~= 1.00 * sine(440 Hz)
          + 0.10 * sine(880 Hz)
          + 0.03 * sine(1320 Hz)

violin A4 ~= 1.00 * sine(440 Hz)
           + 0.55 * sine(880 Hz)
           + 0.35 * sine(1320 Hz)
           + 0.20 * sine(1760 Hz)
```

Both are still `A4`. They differ because their wave shapes differ.

## 3. Timbre

Timbre is the part of sound that lets us hear two instruments as different even
when they play the same note at the same loudness.

For the first model, timbre is:

```text
timbre = harmonic_profile + envelope_profile + variation_profile
```

`harmonic_profile` answers:

> Which sine waves are mixed together?

`envelope_profile` answers:

> How does the loudness change over the life of the note?

`variation_profile` answers:

> What small imperfections make repeated notes less machine-perfect?

V1 may implement harmonics and a simple envelope only. Variation should be
specified now so future implementations do not need to redesign the model.

## 4. Instrument Model

Every implementation should expose this conceptual type:

```text
InstrumentProfile(
    id,
    display_name,
    synthesis_kind,
    gain,
    harmonic_profile,
    envelope_profile,
    variation_profile,
)
```

Fields:

| Field | Meaning |
|-------|---------|
| `id` | stable machine-readable name such as `flute_naive` |
| `display_name` | friendly name such as `Naive Flute` |
| `synthesis_kind` | `sine`, `additive`, `sample`, or `physical` |
| `gain` | normalized output gain in `[0.0, 1.0]` |
| `harmonic_profile` | harmonic multipliers and amplitudes |
| `envelope_profile` | attack/decay/sustain/release shape |
| `variation_profile` | optional deterministic or seeded imperfections |

Required V1 synthesis kinds:

| Kind | Meaning |
|------|---------|
| `sine` | one pure sine at the note frequency |
| `additive` | a sum of sine partials derived from the note frequency |
| `silence` | produces no sound; useful for tests and muted tracks |

Reserved future synthesis kinds:

| Kind | Meaning |
|------|---------|
| `sample` | playback from recorded or generated sample data |
| `physical` | sound from a simulated physical model |
| `external` | host/runtime-provided instrument |

## 5. Harmonic Profile

A harmonic profile is a list of partials:

```text
HarmonicPartial(
    frequency_multiplier,
    amplitude,
    phase_cycles = 0.0
)
```

For ordinary harmonic instruments, `frequency_multiplier` is an integer:

```text
1.0, 2.0, 3.0, 4.0, ...
```

The rendered signal before envelope is:

```text
raw(t) =
    sum(
        partial.amplitude
      * sin(2*pi*(fundamental_hz * partial.frequency_multiplier * t
      + partial.phase_cycles))
    )
```

Rules:

- `frequency_multiplier` must be finite and greater than zero
- `amplitude` must be finite and non-negative
- `phase_cycles` must be finite
- the profile must contain at least one partial unless the instrument is
  `silence`
- implementations must normalize or otherwise safely bound the sum before PCM
  encoding
- renderers must skip or reject partials above Nyquist for the active sample
  rate

Skipping high partials is acceptable for V1 because a high note played through a
rich instrument may have upper harmonics that cannot be represented at the
chosen sample rate.

## 6. Envelope Profile

An envelope is a loudness shape over the lifetime of a note.

Required V1 envelope:

```text
ADSREnvelope(
    attack_seconds,
    decay_seconds,
    sustain_level,
    release_seconds
)
```

The sustain section lasts for the remaining note body:

```text
body_seconds = max(0, duration_seconds - attack_seconds - decay_seconds)
```

The release section is a tail after the notated duration. Renderers may either:

- include release samples in the rendered event, or
- mix release into the following timeline when a note-off model exists

V1 should use the simpler rule:

```text
rendered_duration = duration_seconds + release_seconds
```

Envelope stages:

| Stage | Meaning |
|-------|---------|
| attack | fade from `0.0` to `1.0` |
| decay | fade from `1.0` to `sustain_level` |
| sustain | hold `sustain_level` |
| release | fade from current level to `0.0` |

Rules:

- times must be finite and non-negative
- `sustain_level` must be in `[0.0, 1.0]`
- envelopes must never return negative gain
- zero-length attack/decay/release are allowed
- release tails must count against sample budgets

Beginner examples:

```text
piano:  fast attack, medium decay, low sustain, medium release
flute:  slow attack, short decay, high sustain, short release
violin: medium attack, short decay, high sustain, medium release
pluck:  fast attack, long decay, low sustain, short release
```

## 7. Variation Profile

Real instruments are imperfect. The same player pressing the same note twice
will not produce bit-identical audio.

V1 does not need variation, but the model reserves it:

```text
VariationProfile(
    pitch_jitter_cents = 0.0,
    amplitude_jitter = 0.0,
    timing_jitter_seconds = 0.0,
    harmonic_jitter = 0.0,
    seed = optional integer
)
```

Rules:

- all jitter amounts must be finite and non-negative
- default variation is exactly zero for deterministic tests
- seeded variation must be reproducible
- unseeded variation must not be used in unit tests

Variation is where future virtual instruments can gain life without immediately
needing a physical model.

## 8. Required Naive Presets

Every implementation should provide the same small preset library. The numbers
below are deliberately simple teaching values, not claims about real instrument
acoustics.

### Pure Sine

```text
id: sine
kind: sine
harmonics: 1:1.00
envelope: attack=0.005 decay=0.010 sustain=0.90 release=0.030
```

### Naive Flute

```text
id: flute_naive
kind: additive
harmonics: 1:1.00, 2:0.10, 3:0.03
envelope: attack=0.080 decay=0.050 sustain=0.85 release=0.080
```

### Naive Clarinet

```text
id: clarinet_naive
kind: additive
harmonics: 1:1.00, 3:0.45, 5:0.20, 7:0.08
envelope: attack=0.040 decay=0.060 sustain=0.75 release=0.070
```

### Naive Violin

```text
id: violin_naive
kind: additive
harmonics: 1:1.00, 2:0.55, 3:0.35, 4:0.20, 5:0.12
envelope: attack=0.070 decay=0.080 sustain=0.80 release=0.120
```

### Naive Piano

```text
id: piano_naive
kind: additive
harmonics: 1:1.00, 2:0.45, 3:0.25, 4:0.14, 5:0.08
envelope: attack=0.005 decay=0.350 sustain=0.18 release=0.180
```

### Naive Plucked String

```text
id: pluck_naive
kind: additive
harmonics: 1:1.00, 2:0.60, 3:0.38, 4:0.22, 5:0.13, 6:0.08
envelope: attack=0.003 decay=0.450 sustain=0.05 release=0.080
```

## 9. Text Score Integration

`MUS04` instrument declarations should accept a profile reference:

```text
instrument lead profile=violin_naive gain=0.08
instrument bass profile=pluck_naive gain=0.06
```

For custom inline additive instruments, use explicit properties:

```text
instrument bell kind=additive gain=0.06 harmonics=1:1.0,2:0.4,3:0.2 envelope=0.002,0.6,0.1,0.4
```

The `envelope` shorthand means:

```text
attack_seconds,decay_seconds,sustain_level,release_seconds
```

Rules:

- `profile=<id>` references a known preset or user-defined instrument profile
- `kind=sine` may omit `harmonics`
- `kind=additive` must provide `harmonics` unless `profile` supplies them
- `gain` multiplies the instrument profile gain
- event `velocity` multiplies the instrument event loudness

Final gain before PCM is:

```text
score_gain * track_gain * instrument_gain * event_velocity * envelope(t)
```

If a package does not yet support score or track gain, it should treat the
missing values as `1.0`.

## 10. Rendering Pipeline

For each note event:

1. Convert the note to a fundamental frequency using `MUS00`.
2. Resolve the track's instrument profile.
3. Build one oscillator per harmonic partial.
4. Evaluate and sum the oscillators at each sample time.
5. Apply the envelope at each sample time.
6. Apply gain and velocity.
7. Mix the event into the score timeline.
8. Clamp or normalize to PCM safely.

The conceptual signal is:

```text
instrument_note(t) =
    envelope(t)
  * gain
  * sum(partial_i(t))
```

Where:

```text
partial_i(t) =
    partial_i.amplitude
  * sin(2*pi*(fundamental_hz * partial_i.frequency_multiplier * t
  + partial_i.phase_cycles))
```

## 11. Virtual Keyboard Integration

A virtual keyboard should be able to say:

```text
keyboard.set_instrument("violin_naive")
keyboard.note_on("A4", velocity=0.8)
keyboard.note_off("A4")
```

This requires a runtime note state model:

```text
ActiveVoice(
    voice_id,
    note,
    instrument_id,
    start_time_seconds,
    velocity,
    phase_state,
    envelope_state
)
```

V1 package implementations may render fixed-duration score events first. A
real-time keyboard package will later need note-on/note-off, voice allocation,
voice stealing, and release tails.

## 12. Virtual Orchestra Integration

A virtual orchestra is the same model at a larger scale:

```text
Conductor(
    score,
    tempo_map,
    tracks,
    instruments,
    transport_position,
)
```

The conductor's job is scheduling, not sound physics:

- decide when each event should begin
- send the event to the correct instrument
- apply tempo changes
- mix tracks
- expose play, pause, seek, and stop

Instrument profiles should remain independent of the conductor so a flute can
be reused by a keyboard, a score renderer, or a sequencer.

## 13. Future Physical Models

This spec intentionally leaves a bridge to physical modeling.

Future `physical` instruments may model:

- string displacement and tension
- fret position and string length
- bow pressure and bow velocity
- reed stiffness and airflow
- tube resonance
- soundboard coupling
- body resonance
- microphone or pickup position
- material imperfections

These models may need symbolic computation, differential equations, numerical
integration, or finite-difference simulation. They should still lower into the
same shared signal contract:

```text
InstrumentVoice.value_at(time_seconds) -> float
```

That means a physical violin can eventually replace `violin_naive` without
changing the score, keyboard, conductor, DAC, or audio sink layers.

## 14. Safety and Resource Limits

Implementations must enforce:

- maximum partial count per instrument
- maximum rendered sample count including release tails
- maximum simultaneous voices
- maximum track count
- maximum event count
- finite numeric values for all gains, times, phases, and jitter settings
- gain and velocity bounds before mixing

Renderers should reject profiles that would create unbounded work. For example,
an additive instrument with 100,000 harmonics must fail during validation rather
than during rendering.

## 15. Testing Requirements

Every implementation should test:

- same note has the same fundamental frequency for all instruments
- different harmonic profiles produce different sample sequences
- additive synthesis sums partials correctly on a tiny known example
- ADSR envelope values at attack, decay, sustain, and release points
- preset profiles are present and valid
- high harmonics above Nyquist are skipped or rejected deterministically
- release tails contribute to rendered duration and sample budgets
- invalid profiles fail before rendering
- seeded variation is deterministic when variation is implemented

Golden fixtures should include:

- one pure sine note
- one flute-like note
- one violin-like note
- one piano-like note
- one two-instrument score

The tests should compare broad numerical properties and small deterministic
fixtures. They should not claim that the naive presets sound exactly like real
instruments.

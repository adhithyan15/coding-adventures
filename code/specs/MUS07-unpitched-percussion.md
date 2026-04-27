# MUS07: Unpitched Percussion and Drum Kit Events

## 1. Overview

`MUS05` defines pitched instruments as:

```text
note frequency + instrument profile -> shaped waveform
```

That model works for:

- melodic instruments such as flute, violin, and piano
- pitched percussion such as glockenspiel, marimba, vibraphone, and timpani

But it does not cleanly describe most drum-kit and percussion sounds.

A snare is not naturally "A4".
A hi-hat is not naturally "C#5".
A shaker is not best understood as "one stable frequency plus harmonics".

This spec defines the next sound family:

```text
percussion hit + kit voice profile -> transient + noise + resonances -> shaped waveform
```

The goal is to make drum machines, virtual kits, groove tracks, and eventually
keyboard rhythm sections possible without pretending every percussive sound is
just another note.

## 2. Beginner Mental Model

There are now two different musical event families in the stack:

```text
pitched event      -> "play A4 on flute"
percussion event   -> "hit snare now"
```

For a pitched event, the core question is:

> What frequency is this note?

For an unpitched percussion event, the core question is:

> What was struck, how hard, and how does that object ring and decay?

So the first beginner distinction is:

| Family | Main identity |
|--------|----------------|
| pitched instruments | note name / frequency |
| unpitched percussion | hit identity / excitation shape |

This is why a piano roll and a drum machine feel related but not identical.

## 3. Why Notes Are Not Enough

The note-based model assumes:

- a stable fundamental frequency
- optionally related harmonics above it
- a sustain portion that remains "the same note"

Many percussion sounds violate at least one of those assumptions:

- **snare**: noisy attack plus short body resonance
- **kick**: very low thump plus click transient, often with pitch dropping over time
- **hi-hat**: bright noisy metallic burst, often with no stable pitch center
- **crash cymbal**: broadband splash with long, noisy decay
- **clap**: multiple short transients rather than one clean sustained tone
- **shaker**: repeated tiny stochastic impacts inside one gesture

So the core abstraction changes from:

```text
frequency-centered
```

to:

```text
excitation-centered
```

## 4. Conceptual Signal Model

The first unpitched percussion model should be:

```text
hit event
  -> excitation source
  -> optional resonant modes
  -> amplitude envelope
  -> sampled PCM
```

The components are:

### 4.1 Excitation Source

The excitation is the initial energy injection:

- impulse / click
- short burst of filtered noise
- short tonal burst
- mixed excitation

Examples:

- kick: click + low burst
- snare: click + noise burst
- hi-hat: bright noise burst
- clap: several closely spaced bursts

### 4.2 Resonant Modes

After excitation, the struck body may ring in one or more modes:

- drum shell modes
- membrane/body resonance
- metallic modes
- wood block modes

Unlike simple harmonic instruments, these modes do not have to be integer
multiples of one base frequency.

### 4.3 Envelope

The envelope is still important, but percussion emphasizes:

- attack shape
- very fast decay
- optional noisy tail
- optional choke behavior

The sustain concept is often weak or absent.

## 5. Core Model

Every implementation should expose equivalent conceptual shapes.

### 5.1 Percussion Hit Event

```text
PercussionHitEvent(
    track_id,
    start_tick,
    duration_tick,
    hit_id,
    velocity,
    variation,
)
```

Fields:

| Field | Meaning |
|-------|---------|
| `track_id` | which percussion track the hit belongs to |
| `start_tick` | when the hit begins |
| `duration_tick` | scheduling width or nominal gate length |
| `hit_id` | machine-readable hit name such as `kick` or `snare` |
| `velocity` | normalized hit intensity in `[0.0, 1.0]` |
| `variation` | optional future humanization / articulation selector |

`duration_tick` remains in the model even when the sound is mostly transient,
because:

- the text format already reasons in timed events
- some hits need explicit choke/gate behavior
- later electronic percussion may use duration more strongly

For many acoustic-style kit voices, the physical tail is still mostly defined by
the voice profile itself rather than by the notated duration.

### 5.2 Percussion Voice Profile

```text
PercussionVoiceProfile(
    id,
    display_name,
    family,
    gain,
    excitation_profile,
    resonance_profile,
    envelope_profile,
    choke_group,
    variation_profile,
)
```

Fields:

| Field | Meaning |
|-------|---------|
| `id` | stable machine-readable name such as `snare_naive` |
| `display_name` | friendly name |
| `family` | `kick`, `snare`, `hat`, `cymbal`, `tom`, `hand_percussion`, etc. |
| `gain` | normalized output gain |
| `excitation_profile` | how the hit starts |
| `resonance_profile` | what rings afterward |
| `envelope_profile` | how the energy dies away |
| `choke_group` | optional muting group such as hi-hat open/closed |
| `variation_profile` | future deterministic variation knobs |

### 5.3 Drum Kit Profile

```text
DrumKitProfile(
    id,
    display_name,
    voices_by_hit_id,
)
```

A drum kit maps logical hit names such as `kick`, `snare`, and
`closed_hihat` to actual voice profiles.

This lets the score talk about musical intent:

```text
kick
snare
closed_hihat
open_hihat
ride
crash
```

without hard-coding one exact synthesis recipe forever.

## 6. Starter Hit Vocabulary

Every implementation should support the same first unpitched hit ids:

| Hit Id | Meaning |
|--------|---------|
| `kick` | bass drum / kick |
| `snare` | snare drum |
| `closed_hihat` | closed hi-hat |
| `open_hihat` | open hi-hat |
| `pedal_hihat` | foot hi-hat |
| `low_tom` | low tom |
| `mid_tom` | middle tom |
| `high_tom` | high tom |
| `crash` | crash cymbal |
| `ride` | ride cymbal |
| `clap` | hand clap |
| `cowbell` | cowbell |
| `tambourine` | tambourine |
| `shaker` | shaker |
| `woodblock` | wood block |

Implementations may add more, but these names should be the portable baseline.

## 7. Hi-Hat Choke Semantics

Some percussion hits interact in ways ordinary pitched notes do not.

The first required interaction is hi-hat choking:

- `closed_hihat` should stop or strongly damp `open_hihat`
- `pedal_hihat` should stop or strongly damp `open_hihat`
- repeated `open_hihat` hits should not accumulate unrealistically forever

This is modeled through `choke_group`.

Example:

```text
choke_group = "hihat"
```

Rules:

- voices in the same choke group may silence or sharply attenuate each other
- the exact attenuation curve may be implementation-defined in V1
- deterministic implementations must document the chosen rule

## 8. Portable Score Integration

`MUS04` defines `music-machine-score/v2`, which currently supports:

- `note`
- `rest`

This spec does not retroactively change V2.
Instead, it defines the next required event form for a future portable-score
revision:

```text
event <track_id> <start_tick> <duration_tick> hit <hit_id> [velocity=<0.0..1.0>]
```

Examples:

```text
event drums 0 120 hit kick velocity=0.90
event drums 240 120 hit snare velocity=0.80
event drums 120 60 hit closed_hihat velocity=0.55
event drums 360 240 hit open_hihat velocity=0.60
```

The corresponding percussion instrument declaration should look like:

```text
instrument drums kind=drumkit kit=standard gain=0.12
```

Conceptually:

| Instrument kind | Meaning |
|-----------------|---------|
| `sine` | pitched oscillator voice |
| `silence` | silent voice |
| `external` | host-provided pitched voice |
| `drumkit` | unpitched percussion kit voice set |

Portable score rules for `hit` events:

- `hit_id` must be one of the portable baseline hits or another declared kit hit
- `velocity` remains normalized in `[0.0, 1.0]`
- `duration_tick` must be positive
- `hit` events may overlap freely unless choke semantics say otherwise
- `hit` events must only target tracks bound to percussion-capable instruments

## 9. Rendering Semantics

The renderer lowers a percussion hit in these phases:

1. Convert `start_tick` and `duration_tick` into score time.
2. Resolve `hit_id` through the active drum kit.
3. Render excitation + resonances + envelope into floating samples.
4. Apply choke-group rules if overlapping hits require it.
5. Mix the result into the shared PCM timeline.

V1 requirements:

- renderers must support a deterministic standard drum kit
- renderers must support at least one noisy percussion source
- renderers must support at least one resonant low-drum source
- renderers must support hi-hat choke behavior
- renderers must keep sample budgets bounded just like pitched rendering
- renderers must safely clamp mixed PCM output

## 10. Relationship to Pitched Percussion

Pitched percussion remains in the pitched world.

Examples:

- glockenspiel
- vibraphone
- marimba
- xylophone
- tubular bells
- timpani

These still answer:

> Which note is this?

Unpitched percussion answers:

> Which thing was hit?

Some instruments sit near the boundary:

- timpani is clearly pitched
- toms are weakly pitched in real life but should begin in the percussion model
- steel drums are pitched enough to stay in the pitched model
- melodic toms may later support both note and hit views

So the beginner rule is:

- if the instrument is usually played as notes on a scale, keep it pitched
- if the instrument is usually played as kit hits or rhythmic gestures, use the
  percussion model

## 11. Standard Kit

Every implementation should eventually provide:

```text
standard_kit
```

with at least:

- kick
- snare
- closed_hihat
- open_hihat
- pedal_hihat
- low_tom
- mid_tom
- high_tom
- crash
- ride

Optional early additions:

- clap
- cowbell
- tambourine
- shaker
- woodblock

This kit is the percussion equivalent of the beginner melodic preset library in
`MUS05`.

## 12. Future Growth

This model leaves room for better realism later:

- filtered-noise excitation
- pitch-drop envelopes for kick drums
- multi-burst clap and shaker models
- cymbal noise bands and metallic mode banks
- sampled attacks for hybrid percussion
- physically modeled membranes and resonant shells
- symbolic rhythm transcription from recorded audio

The point of this spec is not to solve all percussion physics now.
It is to define the right abstraction boundary so later work can grow without
forcing everything back into fake note names.

## 13. Implementation Plan

Recommended rollout:

1. spec and fixture layer:
   - define portable hit ids and standard kit semantics
2. naive percussion package:
   - deterministic `standard_kit`
   - kick / snare / hi-hat / tom / cymbal starter voices
3. music-machine integration:
   - parse and render `hit` events
   - enforce track/kit compatibility
4. demo layer:
   - mixed band + drum groove examples
5. later realism:
   - better noise shaping, choke behavior, and physical models

## 14. Summary

The music stack now has two core sound families:

```text
pitched:
    note -> frequency -> instrument timbre -> PCM

percussion:
    hit -> excitation/resonance/envelope -> PCM
```

That split is the key conceptual step needed for drum machines, rhythm tracks,
and eventually realistic virtual kits.

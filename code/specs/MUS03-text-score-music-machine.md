# MUS03: Text Score and Music Machine

## 1. Overview

This spec defines the first text-based sheet music format for the music stack
and the first "music machine" that can play a sequence of notes.

Earlier specs built the lower layers:

```text
MUS00: note name -> frequency
OSC00: oscillator -> sampled values
MUS01: note -> PCM -> virtual speaker
MUS02: PCM -> real audio device sink
```

This spec adds the layer a beginner can write by hand:

```text
text score -> timed note/rest events -> rendered PCM -> optional playback sink
```

The first target song is "Happy Birthday" because it is short, familiar, and
mostly monophonic.

## 2. Beginner Mental Model

A single note such as `A4` answers "what pitch?"

A score answers "what pitch, for how long, and in what order?"

For example:

```text
tempo: 120

G4/e G4/e A4/q G4/q C5/q B4/h
```

Read this as:

- play `G4` for an eighth note
- play another `G4` for an eighth note
- play `A4`, `G4`, and `C5` for quarter notes
- play `B4` for a half note

The music machine turns each text token into a timed event, renders each note
through the existing note-to-sound chain, inserts silence for rests, then joins
the PCM buffers together.

## 3. Scope

V1 is intentionally small:

- one melody line
- one tempo for the whole score
- one amplitude for the whole score
- one sample rate for the whole score
- notes and rests only
- blocking playback only when an audio sink is explicitly requested

Out of scope for V1:

- chords
- overlapping notes
- multiple voices
- MIDI input
- key signatures
- automatic beaming
- repeats
- ties
- lyrics
- articulation marks
- real-time keyboard note-on/note-off events

Those belong in later specs. V1 is the simplest bridge from text to sound.

## 4. Text Format

The format is line-oriented and beginner-friendly.

Blank lines are ignored. Lines starting with `#` are comments.

Metadata directives use `name: value`:

```text
title: Happy Birthday
tempo: 120
meter: 3/4
amplitude: 0.18
sample_rate: 44100
```

Music tokens use:

```text
pitch/duration
```

Examples:

```text
C4/q
F#4/e
Bb3/h
R/q
rest/e
```

Barlines may be written as `|`. They are visual separators in V1:

```text
G4/e G4/e | A4/q G4/q C5/q | B4/h
```

The parser should keep enough position information to produce useful error
messages, but V1 does not need a full source-map model.

## 5. Duration Symbols

Duration symbols are measured relative to a quarter note.

At `tempo: 120`, one quarter note lasts:

```text
60 / 120 = 0.5 seconds
```

Required symbols:

| Symbol | Name | Beats |
|--------|------|-------|
| `w` | whole | `4.0` |
| `h` | half | `2.0` |
| `q` | quarter | `1.0` |
| `e` | eighth | `0.5` |
| `s` | sixteenth | `0.25` |

A duration may have one dot suffix:

```text
q. = 1.5 beats
h. = 3.0 beats
e. = 0.75 beats
```

V1 should reject unknown symbols such as `thirtysecond`, multiple dots such as
`q..`, and zero/negative durations.

## 6. Score Model

The parser produces:

```text
TextScore(
    title,
    tempo_bpm,
    meter,
    amplitude,
    sample_rate_hz,
    events,
)
```

Each event is:

```text
ScoreEvent(
    kind = note | rest,
    note = optional note string,
    duration_symbol,
    beat_count,
    duration_seconds,
    source_token,
)
```

Rules:

- `tempo_bpm` must be finite and greater than zero
- `amplitude` must be finite and in `[0.0, 1.0]`
- `sample_rate_hz` must be integer-valued and greater than zero
- note tokens must follow `MUS00`
- rest tokens are `R` or `rest`, case-insensitive
- scores must contain at least one event

Parser resource limits:

- parsers must reject score text above an implementation-defined maximum size
- parsers must reject lines above an implementation-defined maximum length
- parsers must reject event counts above an implementation-defined maximum count

These limits exist because rendering has a sample-budget guard, but parsing
happens first. A score with millions of tiny events should fail while it is
still text, before it can allocate a giant event list.

## 7. Rendering

Rendering a score to PCM is a sequential process.

For a note event:

```text
render_note_to_sound_chain(note, duration_seconds, amplitude, sample_rate_hz)
```

For a rest event:

```text
append zero-valued PCM samples for duration_seconds
```

The output is:

```text
RenderedScore(
    score,
    pcm_buffer,
    rendered_notes,
)
```

`rendered_notes` keeps the inspectable `RenderedNote` objects for note events.
Rests do not have oscillators, so they only contribute silence to the final PCM
buffer.

V1 should not try to remove clicks between adjacent notes. Envelope shaping is a
future layer.

## 8. Playback

Playback is optional and should be a thin convenience:

```text
play_score_text(text)
```

This should:

1. parse the text score
2. render the score to a PCM buffer
3. import the audio device sink lazily
4. call `play_pcm_buffer(...)`

The lazy import matters because parser and renderer tests should not require a
native audio backend.

## 9. Happy Birthday V1 Fixture

The first bundled score should be a simple monophonic Happy Birthday melody:

```text
title: Happy Birthday
tempo: 120
meter: 3/4
amplitude: 0.18
sample_rate: 44100

G4/e G4/e | A4/q G4/q C5/q | B4/h R/q |
G4/e G4/e | A4/q G4/q D5/q | C5/h R/q |
G4/e G4/e | G5/q E5/q C5/q | B4/q A4/q R/q |
F5/e F5/e | E5/q C5/q D5/q | C5/h
```

This is not trying to be a publishing-grade engraving of the song. It is a
small score that proves the machine can go from human-readable note text to
audible output.

## 10. Testing Requirements

The first package must test:

- metadata parsing
- note event parsing
- rest event parsing
- duration math at a known tempo
- dotted duration math
- useful errors for malformed tokens
- parser resource limits
- rendering produces the expected sample count
- rests produce zero PCM samples
- Happy Birthday parses and renders
- playback delegates to an injected or monkeypatched sink without requiring a
  real audio device in unit tests

Coverage should exceed the repo threshold and preferably stay above `95%`.

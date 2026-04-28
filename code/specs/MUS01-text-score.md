# MUS01: Text-Based Score Format

## 1. Overview

This spec defines a tiny, text-first score format for writing melody as human-readable
notes plus durations. It is the next layer after:

- `MUS00` for note-to-frequency mapping
- `OSC00` for oscillator/sampler abstractions

The goal is to provide a lightweight file format that:

- is easy for people to write by hand
- is trivial to generate programmatically
- can be consumed by a music machine that renders samples or feeds an instrument layer

The package for this spec is initially a parser + renderer for simple monophonic lines.

## 2. Core Abstractions

The format represents a piece as:

- optional directives (`tempo`, `title`)
- one event per non-comment line

Each event uses:

```text
<note-or-rest> <duration_beats> [<velocity>]
```

Where:

- `<note-or-rest>` is `A4`, `C#5`, `Bb3`, etc., or `R` for a rest
- `<duration_beats>` is a positive float
- `<velocity>` is an optional float in the range `[0.0, 1.0]` (defaults to `1.0`)

## 3. Example Score

```text
title: Happy Birthday
tempo: 120

A4 0.5
A4 0.5
G4 1.0
A4 1.0
C5 1.0
B4 1.0

R 0.5
```

At `tempo: 120`, one beat is `0.5` seconds.

## 4. Directives

Each score can start with any number of directive lines.

- `tempo: <bpm>`  
  - `<bpm>` must be greater than `0`
  - defaults to `120` if not present
- `title: <text>`
  - optional
  - stored as the score title

Unsupported directives should raise parse errors.

## 5. Comment and Whitespace Rules

- Full-line comments start with `#` after optional indentation.
- Empty lines are skipped.
- Leading/trailing whitespace is ignored.
- Inline comments are not supported initially.

## 6. Timing Model

If a score has tempo `T` BPM, then:

```text
duration_seconds = 60 * beats / T
```

Score and note events should preserve beat values so they can be re-tempo'd later.

## 7. Rendering Model (First Layer)

The first renderer is allowed to be intentionally simple:

- For note events, emit sampled sine wave segments at the note frequency.
- For rest events, emit zero-valued samples.
- Concatenate rendered events in order.

The initial implementation uses one sine oscillator per event and a uniform sample
rate (e.g. `44100` Hz by default).

## 8. Data Model

The package should provide:

- `MusicalEvent`
  - `note: Note | None` (`None` for rests)
  - `duration_beats: float`
  - `velocity: float`
  - `duration_seconds(tempo_bpm)` convenience method
- `Score`
  - `title: str | None`
  - `tempo_bpm: float`
  - `events: list[MusicalEvent]`

## 9. API Surface

- `parse_score(text: str) -> Score`
- `render_score(score: Score, sample_rate_hz: int = 44100) -> list[float]`
- `score_duration_seconds(score: Score) -> float`

## 10. Validation

Parsers and renderers must reject:

- missing event duration
- non-numeric durations
- notes that fail `MUS00` parsing
- beats or tempo values that are not positive
- velocity outside `[0.0, 1.0]`
- non-positive sample rates

The parser should fail fast and report the line number where the error occurred.

## 11. Example Programmatic API

```python
from music_machine import Score, MusicalEvent, parse_score, render_score

score = parse_score("""
title: Happy Birthday
tempo: 120

A4 0.5
A4 0.5
G4 1.0
A4 1.0
C5 1.0
B4 1.0
""")

samples = render_score(score, sample_rate_hz=48000)
print(len(samples))  # total PCM sample count for the score
```

## 12. Out Of Scope (This Version)

- chords and polyphony
- key/time signatures
- swing/tuplet/grid feel
- articulations beyond velocity
- looping/scheduling metadata

These are intentionally deferred to later music/track spec layers.

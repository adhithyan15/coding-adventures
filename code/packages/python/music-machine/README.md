# music-machine

`music-machine` is the first beginner-facing layer above the note-to-sound
stack. It lets a small text score become timed events and then signed 16-bit PCM
audio.

```text
text score -> note/rest events -> instrument renderer -> PCM buffer -> optional audio sink
```

The package intentionally starts with a tiny sheet-music language instead of a
full notation system. V1 supports one melody line, one tempo, one selected
instrument, notes, rests, duration symbols, and visual barlines.

The package also supports the canonical `music-machine-score/v2` format from
`MUS04`. That format is better for programmatic sheet music because every event
has an explicit track, start tick, duration tick, and velocity.

## Text Score Example

```text
title: Tiny Melody
tempo: 120
meter: 4/4
amplitude: 0.18
instrument: flute_naive
sample_rate: 44100

C4/q D4/q E4/q R/q
```

Each music token is `pitch/duration`.

| Symbol | Meaning | Beats |
|--------|---------|-------|
| `w` | whole note | `4.0` |
| `h` | half note | `2.0` |
| `q` | quarter note | `1.0` |
| `e` | eighth note | `0.5` |
| `s` | sixteenth note | `0.25` |

A single dot makes a duration one-and-a-half times as long, so `q.` is `1.5`
beats.

## Instruments

Scores can choose a naive instrument profile by id:

```text
instrument: violin_naive
```

Or they can choose a General-MIDI-style keyboard program number:

```text
program: 74
```

`program: 74` resolves to the current naive flute profile. `instrument:` and
`program:` are mutually exclusive, and both must appear before the music tokens.
If neither is provided, the score defaults to `instrument: sine`.

## Portable Multi-Track Scores

Use `parse_portable_score` and `render_portable_score_to_pcm` for the canonical
multi-track format:

```text
format: music-machine-score/v2
title: Tiny Duet
ppq: 100
sample_rate: 1000
tempo 0 600
meter 0 4/4

instrument lead profile=flute_naive gain=0.5
instrument bass program=33 gain=0.4

track melody instrument=lead
track bassline instrument=bass

event melody 0 100 note A4 velocity=0.8
event melody 100 100 note B4 velocity=0.8
event bassline 0 200 note A2,E3 velocity=0.7
```

`ppq` means pulses per quarter note. A note event can contain one pitch or a
comma-separated chord. Events may overlap across tracks; rendering mixes them
into one signed 16-bit mono `PCMBuffer` and clamps mixed samples safely.

Instrument declarations can reference a naive profile, a General-MIDI-style
program number, or the built-in `kind=sine` / `kind=silence` profiles.

## Usage

```python
from music_machine import HAPPY_BIRTHDAY_TEXT, parse_score, render_score_to_pcm

score = parse_score(HAPPY_BIRTHDAY_TEXT)
rendered = render_score_to_pcm(score)

print(score.title)
print(rendered.pcm_buffer.sample_count())
```

For the portable format:

```python
from music_machine import parse_portable_score, render_portable_score_to_pcm

score = parse_portable_score(score_text)
rendered = render_portable_score_to_pcm(score)
```

Playback is intentionally lazy, because parsing and rendering should work
without a native audio device package:

```python
from music_machine import play_score_text

play_score_text(HAPPY_BIRTHDAY_TEXT)
```

Portable scores use `play_portable_score_text` for the same lazy playback path.

The package also ships a reusable four-part portable demo:

```python
from music_machine import MINI_ORCHESTRA_TEXT, parse_portable_score, render_portable_score_to_pcm

score = parse_portable_score(MINI_ORCHESTRA_TEXT)
rendered = render_portable_score_to_pcm(score)

print(len(score.tracks))         # 4
print(score.title)               # Mini Orchestra
print(rendered.pcm_buffer.sample_count())
```

It also ships a mixed melodic-plus-pitched-percussion demo:

```python
from music_machine import (
    PITCHED_PERCUSSION_MIX_TEXT,
    parse_portable_score,
    render_portable_score_to_pcm,
)

score = parse_portable_score(PITCHED_PERCUSSION_MIX_TEXT)
rendered = render_portable_score_to_pcm(score)

print(score.title)               # Pitched Percussion Mix
print(len(score.tracks))         # 5
print(rendered.pcm_buffer.clipped_sample_count)   # 0
```

For programmatic composition, use `PortableScoreBuilder` instead of hand-writing
every portable `event` line:

```python
from music_machine import PortableScoreBuilder, parse_portable_score

builder = PortableScoreBuilder(title="Tiny Builder Song", ppq=100, sample_rate_hz=2000)
builder.add_tempo(0, 600)
builder.add_instrument("lead", kind="sine", gain=0.5)
builder.add_track("melody", instrument_id="lead")
builder.add_note("melody", 0, 100, "A4", velocity=0.8)
builder.add_chord("melody", 100, 100, ("C5", "E5"), velocity=0.6)
builder.add_rest("melody", 200, 50)

score = builder.build()
score_text = builder.to_text()
round_tripped = parse_portable_score(score_text)
```

For more musical sequencing, the builder now has measure helpers and a phrase
cursor:

```python
from music_machine import PortableScoreBuilder

builder = PortableScoreBuilder(title="Phrase Demo", ppq=120, sample_rate_hz=2000)
builder.add_tempo(0, 600)
builder.add_meter(0, "4/4")
builder.add_instrument("lead", kind="sine", gain=0.5)
builder.add_track("melody", instrument_id="lead")

phrase = builder.phrase("melody", measure_number=2, beat_offset=1.0)
phrase.note("A4", 1.0, velocity=0.8).rest(0.5).chord(("C5", "E5"), 0.5)
```

You can also capture a phrase as a motif, then repeat or transpose it:

```python
motif = phrase.motif()
builder.apply_motif(
    motif,
    "melody",
    builder.measure_start_tick(3),
    transpose_semitones=12,
    repeat_count=2,
)
```

For bigger structure, capture a whole multi-track section and reuse it like a
verse or chorus:

```python
section = builder.capture_section(
    start_tick=builder.measure_start_tick(1),
    end_tick=builder.measure_start_tick(5),
)

builder.apply_section(
    section,
    builder.measure_start_tick(9),
    track_map={"melody": "answer"},
    transpose_semitones={"melody": 12},
    velocity_scale={"bassline": 0.8},
)
```

## Safety Limits

The parser has explicit limits for score size, line length, and event count.
Those checks happen before rendering so a huge text file cannot allocate an
unbounded number of note/rest events before the sample-budget guard has a chance
to run.

## How It Fits

`music-machine` does not define instrument timbres itself. It parses a score,
resolves the selected profile through `musical-instruments`, then renders each
note through the lower layers:

```text
note-frequency -> musical-instruments -> oscillator -> sampler -> pcm-audio
```

Rests are simpler: the machine appends zero-valued PCM samples for the requested
duration. Instrument release tails are mixed into the timeline, so the rendered
PCM buffer may be slightly longer than the sum of notated durations.

## Development

```bash
bash BUILD
```

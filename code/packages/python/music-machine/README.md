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

## Usage

```python
from music_machine import HAPPY_BIRTHDAY_TEXT, parse_score, render_score_to_pcm

score = parse_score(HAPPY_BIRTHDAY_TEXT)
rendered = render_score_to_pcm(score)

print(score.title)
print(rendered.pcm_buffer.sample_count())
```

Playback is intentionally lazy, because parsing and rendering should work
without a native audio device package:

```python
from music_machine import play_score_text

play_score_text(HAPPY_BIRTHDAY_TEXT)
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

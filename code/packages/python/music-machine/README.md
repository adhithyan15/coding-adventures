# music-machine

`music-machine` is the first beginner-facing layer above the note-to-sound
stack. It lets a small text score become timed events and then signed 16-bit PCM
audio.

```text
text score -> note/rest events -> note-audio renders -> PCM buffer -> optional audio sink
```

The package intentionally starts with a tiny sheet-music language instead of a
full notation system. V1 supports one melody line, one tempo, notes, rests,
duration symbols, and visual barlines.

## Text Score Example

```text
title: Tiny Melody
tempo: 120
meter: 4/4
amplitude: 0.18
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

## How It Fits

`music-machine` does not synthesize notes itself. It parses a score, then sends
each note event through `note-audio`, which exposes the lower layers:

```text
note-frequency -> oscillator -> sampler -> pcm-audio -> virtual DAC -> speaker model
```

Rests are simpler: the machine appends zero-valued PCM samples for the requested
duration.

## Development

```bash
bash BUILD
```

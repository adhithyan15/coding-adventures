# Note Frequency

**Layer:** MUS00
**Depends on:** none

This package parses note names such as `A4`, `C#5`, and `Db3` and converts
them into equal-tempered frequencies in Hertz.

## Why This Exists

Humans usually think in note names.

- "Play `A4`"
- "Go up to `C5`"
- "Type the notes to Happy Birthday"

Computers and waveform generators eventually need frequencies.

- `A4` means `440 Hz`
- `A5` means `880 Hz`
- `C4` means about `261.63 Hz`

This package is the bridge between the human label and the physics number.

## Keyboard Mental Model

The 12 notes in one octave are:

```text
C  C#  D  D#  E  F  F#  G  G#  A  A#  B
```

Then the pattern repeats at the next octave. Going up one octave doubles the
frequency.

## API

```python
from note_frequency import Note, parse_note, note_to_frequency

a4 = parse_note("A4")
a4.semitones_from_a4()   # 0
a4.frequency()           # 440.0

note_to_frequency("A5")  # 880.0
note_to_frequency("C4")  # 261.625565...

db4 = Note(letter="D", accidental="b", octave=4)
db4.frequency()          # same pitch as C#4
```

## Supported Inputs

- natural notes: `A4`, `C3`, `G5`
- sharps: `C#4`, `F#5`
- flats: `Db4`, `Bb3`
- lowercase letters are accepted: `a4`, `g#5`

## Out Of Scope For This Version

- note duration
- tempo
- rests
- chords
- double sharps or double flats
- alternate tuning systems

Those belong in higher-level music packages.


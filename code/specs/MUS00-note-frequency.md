# MUS00: Note-to-Frequency Mapping

## 1. Overview

This spec defines the smallest useful abstraction for typed musical notes:

- parse a note name like `A4`, `C#5`, or `Db3`
- understand which keyboard pitch that label refers to
- convert it into an exact frequency in Hertz

This is the first symbolic layer above the existing waveform packages.
Waveform code answers:

> "Given a frequency and a time, what is the wave value?"

This package answers the question that comes first:

> "Given a note name that a human would type, what frequency should we use?"

That lets later packages render melodies such as "Happy Birthday" from typed
note sequences instead of forcing users to think directly in Hertz.

## 2. Keyboard Model

On a keyboard, the note names repeat:

```text
C  C#  D  D#  E  F  F#  G  G#  A  A#  B
```

Then the pattern starts again one octave higher:

```text
C  C#  D  D#  E  F  F#  G  G#  A  A#  B  C
```

Key ideas:

- There are 12 pitch steps before the pattern repeats.
- Each neighboring key is one **semitone** apart.
- Moving up 12 semitones means moving up one **octave**.
- Moving up one octave doubles the frequency.

Example:

- `A3 = 220 Hz`
- `A4 = 440 Hz`
- `A5 = 880 Hz`

## 3. Input Model

The package accepts note strings of the form:

```text
<letter><optional accidental><octave>
```

Where:

- `<letter>` is one of `A B C D E F G`
- accidental is optional and may be:
  - `#` for sharp
  - `b` for flat
- `<octave>` is an integer such as `3`, `4`, or `5`

Examples of valid inputs:

- `A4`
- `C4`
- `C#5`
- `Db3`
- `g4`

Examples of invalid inputs:

- `H4`
- `A`
- `#4`
- `A##4`
- `4A`

## 4. Equal-Tempered Tuning

This package uses the standard modern tuning convention:

- `A4 = 440 Hz`

and the standard 12-tone equal-tempered scale.

In equal temperament:

- each semitone step multiplies frequency by the same amount
- that multiplier is:

$$2^{1/12}$$

So if a note is `n` semitones away from `A4`, its frequency is:

$$f = 440 \cdot 2^{n/12}$$

Examples:

- `n = 0` for `A4`, so `f = 440`
- `n = 12` for `A5`, so `f = 880`
- `n = -12` for `A3`, so `f = 220`

## 5. Semitone Indexing

Within one octave, the package uses this chromatic ordering:

| Index | Note |
|------:|------|
| 0 | C |
| 1 | C# / Db |
| 2 | D |
| 3 | D# / Eb |
| 4 | E |
| 5 | F |
| 6 | F# / Gb |
| 7 | G |
| 8 | G# / Ab |
| 9 | A |
| 10 | A# / Bb |
| 11 | B |

The reference note `A4` therefore has:

- octave = `4`
- chromatic index = `9`

The semitone distance from `A4` is:

$$n = (octave - 4) \cdot 12 + (index - 9)$$

## 6. Core Abstraction

The package exposes a `Note` value object with:

- `letter`
- `accidental`
- `octave`

and derived methods:

- `semitones_from_a4()`
- `frequency()`

Top-level helper functions:

- `parse_note(text) -> Note`
- `note_to_frequency(text) -> float`

## 7. API Contract

### 7.1 `parse_note(text)`

Parses a string into a `Note`.

Behavior:

- accepts uppercase or lowercase note letters
- preserves `#` and `b` accidental meaning
- normalizes the letter to uppercase in the returned object
- raises `ValueError` for malformed note strings

### 7.2 `Note.semitones_from_a4()`

Returns the signed semitone offset from `A4`.

Examples:

- `A4 -> 0`
- `A5 -> 12`
- `A3 -> -12`
- `C4 -> -9`

### 7.3 `Note.frequency()`

Returns the equal-tempered frequency in Hertz as a floating-point number.

Examples:

- `A4 -> 440.0`
- `A5 -> 880.0`
- `A3 -> 220.0`
- `C4 -> 261.625565...`

### 7.4 `note_to_frequency(text)`

Convenience wrapper for:

```text
parse_note(text).frequency()
```

## 8. Validation Rules

The first version intentionally stays small and strict:

- only single accidentals are supported
- no double sharps or double flats
- no whitespace inside the note string
- octave must be present
- only the 12 standard equal-tempered pitch classes are supported

Out of scope for this version:

- alternate tuning systems
- cent offsets or microtones
- note duration
- rests
- chords
- enharmonic spellings like `E#`, `Cb`, `B#`, `Fb`

Those can be added later in higher-level music notation packages.

## 9. Worked Examples

### Example 1: `A4`

- letter = `A`
- accidental = none
- octave = `4`
- index = `9`
- `n = (4 - 4) * 12 + (9 - 9) = 0`
- frequency = `440 * 2^(0/12) = 440 Hz`

### Example 2: `A5`

- same note name, one octave higher
- `n = 12`
- frequency = `440 * 2^(12/12) = 880 Hz`

### Example 3: `C4`

- `C` has index `0`
- `n = (4 - 4) * 12 + (0 - 9) = -9`
- frequency = `440 * 2^(-9/12) = 261.625565... Hz`

### Example 4: `C#4` and `Db4`

- both map to chromatic index `1`
- both therefore produce the same frequency
- this package treats them as different spellings of the same pitch

## 10. Rollout Scope

This abstraction is small enough that it should exist everywhere we offer a
runtime package interface. The goal is not to create fifteen unrelated
implementations. The goal is to create one tiny musical concept with the same
shape in every host ecosystem.

The rollout target for this layer is:

- Python package: `code/packages/python/note-frequency/`
- Go package: `code/packages/go/note-frequency/`
- Ruby package: `code/packages/ruby/note-frequency/`
- Rust package: `code/packages/rust/note-frequency/`
- TypeScript package: `code/packages/typescript/note-frequency/`
- Elixir package: `code/packages/elixir/note-frequency/`
- Lua package: `code/packages/lua/note-frequency/`
- Perl package: `code/packages/perl/note-frequency/`
- Swift package: `code/packages/swift/note-frequency/`
- Java package: `code/packages/java/note-frequency/`
- Kotlin package: `code/packages/kotlin/note-frequency/`
- C# package: `code/packages/csharp/note-frequency/`
- F# package: `code/packages/fsharp/note-frequency/`
- Dart package: `code/packages/dart/note-frequency/`
- Haskell package: `code/packages/haskell/note-frequency/`
- WebAssembly package: `code/packages/wasm/note-frequency/`

`starlark` is intentionally out of scope here because it is used in this repo
for build-rule infrastructure rather than as a user-facing runtime package
ecosystem.

Every implementation should expose the same three ideas:

- parse a typed note label into a structured `Note`
- compute the semitone distance from `A4`
- compute the equal-tempered frequency in Hertz

Language-specific naming can vary slightly to stay idiomatic, but the semantics
must stay aligned with this spec.

## 11. Cross-Language Parity Vectors

Every implementation should agree on these examples within normal
double-precision floating-point tolerance:

| Input | Expected result |
|------:|-----------------|
| `A4` | semitones from `A4` = `0`, frequency = `440.0` |
| `A5` | semitones from `A4` = `12`, frequency = `880.0` |
| `A3` | semitones from `A4` = `-12`, frequency = `220.0` |
| `C4` | semitones from `A4` = `-9`, frequency = `261.6255653005986...` |
| `C#4` | frequency equals `Db4` |
| `g4` | parses successfully and normalizes the letter to uppercase |

Every implementation should also reject these malformed strings:

- `""`
- `A`
- `H4`
- `#4`
- `4A`
- `A##4`
- `Bb`

Every implementation should also reject unsupported spellings that are
syntactically shaped like a note but outside this version's scope, such as:

- `E#4`
- `Cb4`
- `B#3`
- `Fb5`

## 12. Why This Layer Comes First

This package is intentionally small because it is the first symbolic step from
music notation into signal generation.

Later packages can build on it:

- note events add duration and start time
- oscillators turn frequency into a waveform over time
- envelopes shape how notes begin and end
- renderers and audio devices turn sampled waveforms into speaker output

But all of those higher layers still need one simple answer first:

> "When the user types `A4`, what exact frequency do we mean?"

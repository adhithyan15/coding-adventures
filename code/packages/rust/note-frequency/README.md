# rust/note-frequency


Parses typed musical note labels such as `A4`, `C#5`, and `Db3` into a structured
`Note` value and then maps that note to its equal-tempered frequency in Hertz.

This package is the symbolic front door for the music stack:

- users type note names
- this layer turns them into exact pitches
- later oscillator and renderer packages will turn those pitches into sound

The core reference point is `A4 = 440 Hz`, and every semitone step multiplies
frequency by `2^(1/12)`.


## Usage

```rust
let note = note_frequency::parse_note("A4")?;
let frequency = note_frequency::note_to_frequency("C4")?;
```

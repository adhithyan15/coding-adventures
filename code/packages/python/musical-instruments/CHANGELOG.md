# Changelog

## 0.1.0

- Added naive instrument profile types from `MUS05`.
- Added additive synthesis with ADSR envelopes and release tails.
- Added beginner presets for sine, flute, clarinet, violin, piano, plucked
  string, brass, organ, mallet, synth lead, synth pad, and effects.
- Added a full 128-entry General-MIDI-style melodic program catalog that maps
  keyboard programs onto the current naive preset library.
- Added note rendering to floating samples and signed 16-bit mono PCM.

## Unreleased

- Added named naive pitched-percussion profiles for celesta, glockenspiel,
  vibraphone, marimba, xylophone, tubular bells, timpani, and kalimba.
- Updated the General-MIDI-style catalog so pitched-percussion slots resolve to
  those more specific note-based timbres instead of the single generic mallet
  profile.

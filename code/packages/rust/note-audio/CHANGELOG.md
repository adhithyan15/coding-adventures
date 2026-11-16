# Changelog

All notable changes to the `note-audio` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial safe Rust implementation of MUS01 Stages 1-5 (`note-audio`).
- Implemented `NotePitch` supporting note names, parsed `Note`, MIDI note numbers, and direct frequencies.
- Implemented `Waveform` and `OscillatorSignal` supporting Sine, Square, Pulse, Sawtooth, and Triangle waveforms.
- Implemented `NoteEvent` and `RenderedNote` teaching structures.
- Implemented `render_note`, `render_note_with_limit`, and `render_note_to_sound_chain`.
- Enforced `#![forbid(unsafe_code)]` and zero external crate dependencies.
- Defensive verification bounds for CWE-400, CWE-190, and CWE-20 (`MAX_SAMPLE_RATE_HZ`, `MAX_DURATION_SECONDS`, `MAX_SAMPLES`, Nyquist check).
- Formal warrant document `note-audio.warrant` and required capabilities manifest (`[]`).
- Comprehensive test suite with SplitMix64 PBT and hostile input assertions.

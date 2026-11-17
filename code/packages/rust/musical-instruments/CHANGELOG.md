# Changelog

All notable changes to the `musical-instruments` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-11-16

### Changed

- Partial phases are reduced to [0, 1) cycles before `sin` (`phase - floor(phase)`, exact), because `trig.sin` is NaN beyond 2^29 radians (PHY00a 3.4.1, 7); identical in Python, Rust and TypeScript. Last-bit change in long notes.
- Partial sines come from the bit-exact `trig` crate (TRIG-REF-1, PHY00a) instead of
  `f64::sin`, so rendered notes are byte-identical to the Python and TypeScript lanes
  (the shared `music-machine` golden). Note-local times keep every angle far below the
  2^29-radian domain of `trig::sin`.

## [0.1.0] - 2026-11-15

### Added
- Initial safe Rust implementation of `MUS05` (Naive Instrument Models, Timbre, and Note Rendering).
- Full parity with Python reference package:
  - All 21 naive timbre presets (`sine`, `silence`, `flute_naive`, `clarinet_naive`, `violin_naive`, `piano_naive`, `pluck_naive`, `brass_naive`, `organ_naive`, `mallet_naive`, `celesta_naive`, `glockenspiel_naive`, `vibraphone_naive`, `marimba_naive`, `xylophone_naive`, `tubular_bells_naive`, `timpani_naive`, `kalimba_naive`, `synth_lead_naive`, `synth_pad_naive`, `effect_naive`).
  - Full 128 General MIDI program mapping catalog (`GMProgram`, `all_gm_programs`, `get_gm_program`, `instrument_for_gm_program`).
  - Additive harmonic partial modeling with `HarmonicPartial`.
  - 4-stage loudness shaping with `AdsrEnvelope` and release tail duration expansion.
  - Deterministic PRNG with `SplitMix64` and per-note humanization via `VariationProfile`.
  - Continuous instrument voice generation with `InstrumentSignal` implementing `oscillator::ContinuousSignal`.
  - High-level note rendering pipeline via `render_instrument_note` and `render_instrument_note_pcm`.
  - Automatic note parsing and A440 equal-tempered frequency calculation via `Note`, `parse_note`, and `TryIntoNote`.
- Enforced `#![forbid(unsafe_code)]` and zero third-party dependencies.
- Defensive verification bounds for CWE-400, CWE-190, CWE-20, and CWE-674 (`MAX_PARTIALS = 64`, `MAX_NOTE_DURATION_SECONDS = 60.0`, `MAX_SAMPLE_RATE_HZ = 192_000.0`, `MAX_SAMPLE_COUNT = 16_777_216`).
- Formal warrant document `musical-instruments.warrant` and required capabilities manifest.
- Comprehensive test suite covering SplitMix64 property-based testing, hostile input validation, and bit-for-bit golden parity checks against Python values.

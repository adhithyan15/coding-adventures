# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-20

### Added

- Added the initial Python implementation of `MUS01: Note-to-Sound Signal Chain`.
- Added `NoteEvent`, `PCMFormat`, `PCMBuffer`, `ZeroOrderHoldDACSignal`,
  `LinearSpeakerSignal`, and `RenderedNote`.
- Added `render_note_to_sound_chain()` to compose note-frequency, oscillator,
  sampler, PCM encoding, virtual DAC, and virtual speaker layers.
- Added signed 16-bit PCM encoding with explicit clipping counts.
- Added deterministic mono WAV byte/file helpers as an optional sink.
- Added tests for parity vectors, validation, clipping, DAC hold behavior, WAV
  output, and the visible `A4` rendering chain.

### Changed

- Split reusable PCM, virtual DAC, virtual speaker, and WAV behavior into
  dedicated stage packages.
- Kept `note-audio` as the thin teaching/orchestration layer that wires the
  stages together and re-exports the familiar V1 helpers.

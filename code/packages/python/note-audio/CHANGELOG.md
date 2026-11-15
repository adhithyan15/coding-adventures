# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-11-15

### Added

- Security & CWE hardening (MUS-W1-D):
  - CWE-400 (Resource Exhaustion Defense): enforced `MAX_SAMPLE_RATE_HZ = 192_000.0`, `MAX_DURATION_SECONDS = 60.0`, and `MAX_SAMPLES = 16_777_216` (`MAX_SAMPLE_COUNT`) prior to note synthesis and buffer allocation.
  - CWE-190 / CWE-20 (Strict Input Validation): reject `bool` for numeric arguments, reject `NaN`/`±Inf` and out-of-range duration, sample rate, or sample counts.
  - Pure capability declaration: updated `required_capabilities.json` to declare `capabilities: []` matching AST scanner purity.
- Added VER specification `note_audio.warrant` and `code/specs/note-audio.warrant` matching grammar v0 specification.
- Added SplitMix64 deterministic PRNG property-based testing (PBT) and fuzzing suite achieving 100% test coverage.

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

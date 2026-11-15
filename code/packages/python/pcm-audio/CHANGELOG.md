# Changelog

All notable changes to this package will be documented in this file.

## [0.1.1] - 2026-11-15

### Added

- Security & CWE hardening (MUS-W1-D):
  - CWE-400 (Resource Exhaustion Defense): enforced `MAX_SAMPLE_RATE_HZ = 192_000.0`, `MAX_DURATION_SECONDS = 60.0`, `MAX_SAMPLES = 16_777_216`, `MAX_CHANNELS = 8`, and `MAX_WAV_BYTES = 67_108_864` (64 MiB) prior to any buffer allocation.
  - CWE-190 / CWE-20 (Safe Quantization & Input Validation): introduced `_safe_round_sample` with strict boundary clamping and fail-closed non-finite float/bool rejection.
  - Added robust WAV container parser and header validator (`validate_wav_header`, `from_wav_bytes`) with chunk size and boundary verification.
- Added VER specification `pcm_audio.warrant` matching grammar v0 format.
- Added SplitMix64 deterministic PRNG property-based testing (PBT) and fuzzing suite achieving 100% test coverage.

## [0.1.0] - 2026-04-20

### Added

- Added reusable signed 16-bit mono PCM format and buffer types.
- Added normalized float-to-PCM encoding with explicit clipping counts.
- Added a helper for building PCM buffers from raw floating samples.

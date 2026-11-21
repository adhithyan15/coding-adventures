# Changelog

All notable changes to this package will be documented in this file.

## [0.1.2] - 2026-11-21

### Added & Hardened

- VER15 CWE/CVE security hardening (MUS-W8-C):
  - CWE-20 / CWE-190: added O(1) `_safe_repr` error formatting; caught `OverflowError` on `float(...)` conversions (`10**400`, `Fraction(10**400, 3)`) into `ValueError`; rejected subnormal sample rates; checked RIFF declared payload bounds (`8 + riff_payload <= MAX_WAV_BYTES` and `<= total_len`) and rejected duplicate `fmt ` chunks in `validate_wav_header`.
  - CWE-400: hardened `PCMBuffer` and `samples_to_pcm_buffer` against generator/infinite-iterator DoS, lying `__len__` iterables, and `str`/`bytes`/`bytearray`/non-iterable inputs.
  - Added dedicated `tests/test_hostile_inputs.py` and `tests/test_pbt.py` (SplitMix64 via `coding_adventures_test_bootstrap`).

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

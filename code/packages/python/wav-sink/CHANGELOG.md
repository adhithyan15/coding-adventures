# Changelog

All notable changes to this package will be documented in this file.

## 0.1.2 - 2026-11-21

- VER15 CWE/CVE security hardening (MUS-W8-C):
  - CWE-20 / CWE-190: added O(1) `_safe_repr` error formatting; rejected `bool`/non-int tampered `channel_count`, `bit_depth`, `sample_width_bytes`, and `integer_sample_rate`; caught `OverflowError` on `10**400` `sample_rate_hz` and rejected subnormal rates in `to_wav_bytes`.
  - CWE-22 / CWE-400: enforced `os.fspath` returning `str` and `MAX_PATH_LENGTH = 4096` in `validate_wav_path`.
  - Added dedicated `tests/test_hostile_inputs.py` and `tests/test_pbt.py` (SplitMix64 via `coding_adventures_test_bootstrap`).

## 0.1.1 - 2026-11-15

- Security & CWE hardening (MUS-W1-D):
  - CWE-400 (Resource Exhaustion Defense): enforced `MAX_SAMPLE_RATE_HZ = 192_000.0`, `MAX_DURATION_SECONDS = 60.0`, `MAX_SAMPLES = 16_777_216`, `MAX_CHANNELS = 8`, and `MAX_WAV_BYTES = 67_108_864` (64 MiB) strictly before any buffer or bytes allocations.
  - CWE-22 (Path Traversal & Symlink Defense): implemented `validate_wav_path` enforcing safe `.wav`/`.wave` extensions, containment verification under optional `base_dir`, null-byte rejection, and strict rejection of symlinks across target paths and ancestor hierarchies.
  - CWE-190 / CWE-20 (Overflow & Input Validation): added `calc_total_wav_bytes`, `check_wav_size_bounds`, and `read_wav` with file size validation before reading. Strict type checking rejecting `bool`, `NaN`, `±Inf`, and non-positive sample rates.
  - CWE-674 / CWE-1333: audited zero `import re` and zero unbounded recursion.
- Added VER specification `wav_sink.warrant` and synchronized with `code/specs/wav-sink.warrant`.
- Declared required filesystem capabilities (`fs:read`, `fs:write`) in `required_capabilities.json`.
- Added SplitMix64 deterministic PRNG property-based testing and adversarial security test suite achieving 100% test coverage.

## [0.1.0] - 2026-04-20

### Added

- Added deterministic RIFF/WAVE serialization for mono signed 16-bit PCM buffers.
- Added an optional file sink helper for caller-provided output paths.

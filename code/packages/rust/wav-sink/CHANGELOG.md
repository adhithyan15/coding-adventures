# Changelog

All notable changes to the `wav-sink` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-11-15

### Added
- Initial safe Rust implementation of MUS01 Stage 8 (`wav-sink`).
- Implemented `validate_wav_path` enforcing CWE-22 path traversal and symlink defenses.
- Implemented `write_wav` and `read_wav` moving `PcmAudio` buffers to and from files safely.
- Implemented `to_wav_bytes` and `from_wav_bytes` bounded by `MAX_WAV_BYTES = 64 MiB`.
- Enforced `#![forbid(unsafe_code)]` and zero external crate dependencies.
- Defensive verification bounds for CWE-22 and CWE-400 (`take(MAX_WAV_BYTES + 1)`, symlink traversal refusal).
- Formal warrant document `wav-sink.warrant` and required capabilities manifest (`fs:read`, `fs:write`).
- Comprehensive test suite with SplitMix64 PBT and hostile input assertions.

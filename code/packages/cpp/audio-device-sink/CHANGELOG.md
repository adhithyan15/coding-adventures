# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-11-21

### Added

- `required_capabilities.json` (pure computation, zero OS capabilities) and
  local `audio-device-sink.warrant` specification.
- Explicit bounds constants (`kMinSampleRateHz`, `kMaxChannels`,
  `kMaxBlockingDurationSecondsInt`, `kMaxSamples`, `kMaxErrorMessageBytes`,
  `kMaxBackendNameBytes`, and `MIN_SAMPLE_RATE_HZ`, `MAX_SAMPLE_RATE_HZ`,
  `MAX_CHANNELS`, `MAX_SAMPLES`, `MAX_ERROR_MESSAGE_BYTES`,
  `MAX_BACKEND_NAME_BYTES`).
- `detail::bounded_strlen` / `detail::bounded_cstr` / `detail::bounded_str`,
  `PcmFormat::max_blocking_samples()`, `PcmPlaybackBuffer::from_raw()`,
  `PcmPlaybackBuffer::byte_size()`, and explicit `const char*` overloads for
  `AudioSinkError`, `PlaybackReport::for_buffer`, and `NoopAudioSink`.
- VER15 hostile/adversarial test suites (`test_hostile_nullptr_and_cstr`,
  `test_hostile_bounds_and_overflow`, `test_hostile_lifetime_invariants`) and
  256-trial SplitMix64 property-based tests (`test_pbt_splitmix64`), with zero
  `<cmath>` dependency.

### Fixed

- **CWE-20 / CWE-190 / CWE-400**: Replaced floating-point size cap comparison
  with overflow-checked `std::size_t` / `std::uint64_t` arithmetic
  (`detail::checked_mul_size`, `detail::checked_mul_u64`), enforced pre-flight
  `MAX_SAMPLES` and `MIN_SAMPLE_RATE_HZ..=MAX_SAMPLE_RATE_HZ` / `MAX_CHANNELS`
  bounds before allocation, bounded `AudioSinkError` (`MAX_ERROR_MESSAGE_BYTES`)
  and `backend_name` (`MAX_BACKEND_NAME_BYTES`) via `detail::bounded_strlen`
  prior to `std::string` allocation, validated buffer format before
  constructing `backend_name` in `PlaybackReport::for_buffer`, and guarded
  `duration_seconds()` / `frame_count()` against zero/non-finite states.
- **CWE-476**: Guarded `nullptr` `const char*` inputs in `AudioSinkError`,
  `NoopAudioSink`, `PlaybackReport::for_buffer`, and
  `PcmPlaybackBuffer::from_raw`.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `audio-device-sink` crate in
  namespace `ca::audio_device_sink`: backend-neutral PCM playback primitives
  (no device I/O). V1 scope: mono, signed 16-bit PCM.
- `PcmFormat` with `create` / `validate` / `sample_width_bytes` and value
  equality; `PcmPlaybackBuffer` owning a `std::vector<std::int16_t>` whose
  constructor validates the format and the V1 blocking-duration cap.
- `PlaybackReport` + `PlaybackReport::for_buffer`; an `AudioSink` abstract base
  and `NoopAudioSink` (usable polymorphically through an `AudioSink&`).
- `AudioSinkError`, a `std::runtime_error` subclass carrying an `ErrorKind`;
  `what()` includes the Rust `Display` prefix + detail.
- 31 checks mirroring the crate's unit tests, run under every ISO C++ compiler
  via the shared `iso-harness`; also clean under ASan + UBSan.

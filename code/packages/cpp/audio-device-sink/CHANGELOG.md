# Changelog

All notable changes to this package will be documented in this file.

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

# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `audio-device-sink` crate: backend-neutral PCM
  playback primitives (no device I/O). V1 scope: mono, signed 16-bit PCM.
- `AdsPcmFormat` with `ads_pcm_format_new` / `ads_pcm_format_validate` (rate
  1..=384000, mono, 16-bit) and `ads_pcm_format_sample_width_bytes`.
- `AdsPcmPlaybackBuffer` — malloc-owned samples via `ads_pcm_playback_buffer_new`
  (copies + validates format and the V1 blocking-duration cap) paired with
  `ads_pcm_playback_buffer_free`; sample/frame count, emptiness, and duration
  accessors.
- `AdsPlaybackReport` + `ads_playback_report_for_buffer`; the `AdsAudioSink`
  vtable and `ads_noop_audio_sink` test sink.
- `AdsStatus` error codes with `ads_status_label`; failing calls write a
  formatted message into an optional caller buffer (mirroring the Rust error).
- 39 checks mirroring the crate's unit tests (format validation and boundaries,
  buffer construction/copy/duration, the size cap, the report, and the no-op
  sink), run under every ISO C compiler via the shared `iso-harness`; also clean
  under ASan + UBSan.

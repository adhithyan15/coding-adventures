# Changelog

All notable changes to this package will be documented in this file.

## [0.2.0] - 2026-11-21

### Added

- `required_capabilities.json` (pure computation, zero OS capabilities) and
  local `audio-device-sink.warrant` specification.
- Explicit bounds constants (`ADS_MIN_SAMPLE_RATE_HZ`, `ADS_MAX_CHANNELS`,
  `ADS_MAX_BLOCKING_DURATION_SECONDS_INT`, `ADS_MAX_SAMPLES`) and
  `ads_pcm_playback_buffer_byte_size`.
- VER15 hostile/adversarial test suites (`test_hostile_null_pointers`,
  `test_hostile_bounds_and_overflow`, `test_hostile_double_free_and_uaf`) and
  256-trial SplitMix64 property-based tests (`test_pbt_splitmix64`), with zero
  `<math.h>` / `-lm` dependency.

### Fixed

- **CWE-476**: Guarded all NULL pointer paths (`out == NULL` in
  `ads_pcm_format_new` / `ads_pcm_playback_buffer_new`, `samples == NULL` when
  `n > 0`, `NULL` structs in `ads_pcm_playback_buffer_free` and all accessors,
  `NULL` `self`/`buffer`/`report`/`backend_name` in `ads_playback_report_for_buffer`
  and `noop_play_blocking`).
- **CWE-190 / CWE-400**: Replaced floating-point size cap comparison with
  overflow-checked `size_t` / `uint64_t` arithmetic (`checked_mul_size`,
  `checked_mul_u64`), enforced pre-flight `ADS_MAX_SAMPLES` and
  `ADS_MIN_SAMPLE_RATE_HZ..=ADS_MAX_SAMPLE_RATE_HZ` caps before allocation, and
  guarded against division-by-zero / non-finite durations on zeroed structs.
- **CWE-415 / CWE-416**: Zero-on-free wipe of owned sample bytes and full
  struct zeroing in `ads_pcm_playback_buffer_free` (idempotent double-free
  safety and safe use-after-free rejection).

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

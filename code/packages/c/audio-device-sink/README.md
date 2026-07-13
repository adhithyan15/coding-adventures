# audio-device-sink (C)

**Backend-neutral PCM playback primitives** in pure ISO C17. A faithful port of
the Rust [`audio-device-sink`](../../rust/audio-device-sink) crate.

Intentionally boring in the best way: it does **not** open devices, talk to Core
Audio, parse notes, or generate waves. It defines the shared contract every real
audio backend must obey — a validated PCM format, an owned playback buffer, a
playback report, and a sink "trait" (here a small vtable), plus a no-op sink for
tests.

V1 scope (matching the Rust crate): **mono, signed 16-bit PCM only.**

## API

```c
#include "audio_device_sink.h"

AdsPcmFormat fmt;
char msg[128];
if (ads_pcm_format_new(48000, 1, 16, &fmt, msg, sizeof msg) != ADS_OK) { /* msg */ }

int16_t pcm[4] = {0, 100, -100, 32000};
AdsPcmPlaybackBuffer buf;
ads_pcm_playback_buffer_new(pcm, 4, fmt, &buf, msg, sizeof msg);   /* copies pcm */
double secs = ads_pcm_playback_buffer_duration_seconds(&buf);      /* 4/48000 */

AdsAudioSink sink = ads_noop_audio_sink("noop");
AdsPlaybackReport report;
sink.play_blocking(&sink, &buf, &report, NULL, 0);
ads_pcm_playback_buffer_free(&buf);
```

- **`AdsPcmFormat`** — `ads_pcm_format_new` / `ads_pcm_format_validate` (rate
  1..=384000, mono, 16-bit) and `ads_pcm_format_sample_width_bytes`.
- **`AdsPcmPlaybackBuffer`** — malloc-owned samples;
  `ads_pcm_playback_buffer_new` (copies + validates the format and the V1
  duration cap) paired with `_free`; `_sample_count` / `_frame_count` /
  `_is_empty` / `_duration_seconds`.
- **`AdsPlaybackReport`** + `ads_playback_report_for_buffer`.
- **`AdsAudioSink`** — a vtable (`play_blocking`); `ads_noop_audio_sink` is the
  test sink.
- **Errors** — `AdsStatus` (`ADS_ERR_INVALID_FORMAT`, `ADS_ERR_INVALID_SAMPLES`,
  ...) plus `ads_status_label`; failing calls write a message into an optional
  caller buffer (mirroring the Rust error's formatted string).

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.

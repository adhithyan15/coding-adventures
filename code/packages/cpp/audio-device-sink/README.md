# audio-device-sink (C++)

**Backend-neutral PCM playback primitives**, header-only, pure ISO C++17. A
faithful port of the Rust [`audio-device-sink`](../../rust/audio-device-sink)
crate, in namespace `ca::audio_device_sink`.

Intentionally boring: it does **not** open devices, talk to Core Audio, parse
notes, or generate waves. It defines the shared contract every real audio
backend must obey. V1 scope (matching the Rust crate): **mono, signed 16-bit
PCM only.**

## API

```cpp
#include "audio_device_sink.hpp"
namespace ads = ca::audio_device_sink;

ads::PcmFormat fmt = ads::PcmFormat::create(48000, 1, 16);   // throws on invalid
ads::PcmPlaybackBuffer buf({0, 100, -100, 32000}, fmt);
double secs = buf.duration_seconds();                        // 4 / 48000

ads::NoopAudioSink sink("noop");
ads::PlaybackReport report = sink.play_blocking(buf);
```

- **`PcmFormat`** — `create(rate, channels, bits)` (validates) / `validate()` /
  `sample_width_bytes()`, with value equality.
- **`PcmPlaybackBuffer`** — owns a `std::vector<std::int16_t>`; its constructor
  validates the format and the V1 duration cap (throws `AudioSinkError`);
  `sample_count()` / `frame_count()` / `is_empty()` / `duration_seconds()`.
- **`PlaybackReport`** + `PlaybackReport::for_buffer`.
- **`AudioSink`** abstract base + `NoopAudioSink` (usable polymorphically
  through an `AudioSink&` — the trait-object analog).
- **`AudioSinkError`** — a `std::runtime_error` subclass carrying an `ErrorKind`
  (`InvalidFormat`, `InvalidSamples`, `UnsupportedPlatform`,
  `BackendUnavailable`, `BackendFailure`); `what()` includes the Rust
  `Display` prefix + detail.

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.

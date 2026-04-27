# rust/audio-device-coreaudio

Core Audio backend for `audio-device-sink`.

This crate is the first real operating-system audio backend in the music stack.
It accepts a validated `PcmPlaybackBuffer` and, on macOS, schedules it through
Core Audio's `AudioQueue` API for blocking playback on the default output
device.

The crate keeps the OS boundary visible:

- `audio-device-sink` defines the trait and PCM types
- `audio-device-coreaudio` implements that trait with Core Audio
- callers can use the trait without knowing how Core Audio works internally

## Example

```rust
use audio_device_coreaudio::CoreAudioSink;
use audio_device_sink::{AudioSink, PcmFormat, PcmPlaybackBuffer};

let format = PcmFormat::new(44_100, 1, 16)?;
let buffer = PcmPlaybackBuffer::new(vec![0; 4410], format)?;
let report = CoreAudioSink::new().play_blocking(&buffer)?;

assert_eq!(report.backend_name, "coreaudio");
```

Non-macOS platforms compile, but non-empty playback returns
`UnsupportedPlatform`.

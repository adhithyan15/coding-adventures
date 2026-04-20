# rust/audio-device-sink

Backend-neutral contract for playing already-rendered PCM audio.

This package is the Rust boundary between the virtual music stack and concrete
operating-system audio backends. It does not know about notes, frequencies,
oscillators, samplers, files, or Core Audio. It only defines:

- `PcmFormat` for the shape of PCM samples
- `PcmPlaybackBuffer` for owned signed 16-bit mono sample data
- `AudioSink` for blocking playback sinks
- `PlaybackReport` and `AudioSinkError` for observable results

## Example

```rust
use audio_device_sink::{
    AudioSink, NoopAudioSink, PcmFormat, PcmPlaybackBuffer,
};

let format = PcmFormat::new(44_100, 1, 16)?;
let buffer = PcmPlaybackBuffer::new(vec![0, 1024, 0, -1024], format)?;
let report = NoopAudioSink::new("teaching-noop").play_blocking(&buffer)?;

assert_eq!(report.frames_played, 4);
```

Real OS output lives in backend crates such as `audio-device-coreaudio`.

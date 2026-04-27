# coding-adventures-audio-device-sink

Rust-backed Python audio device sink for already-rendered PCM buffers.

This package is the first Python-facing bridge from the virtual music pipeline
to a real operating-system audio device. It starts at PCM. It does not parse
notes, build oscillators, sample waves, or encode floats.

## What It Provides

- `play_pcm_buffer(buffer)` for existing `pcm_audio.PCMBuffer` objects
- `play_samples(samples, sample_rate_hz=..., channel_count=1)` for raw PCM
- `PlaybackReport` describing what the sink accepted
- `AudioDeviceError` for readable device/backend failures

V1 supports mono signed 16-bit PCM and blocking playback only. macOS uses the
Rust Core Audio backend. Non-macOS imports still work, but non-empty playback
raises an unsupported-platform error until more backends land.

## Example

```python
from audio_device_sink import play_pcm_buffer
from note_audio import render_note_to_sound_chain

rendered = render_note_to_sound_chain("A4", duration_seconds=0.25, amplitude=0.2)
report = play_pcm_buffer(rendered.pcm_buffer)

print(report.frames_played, report.backend_name)
```

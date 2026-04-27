# Changelog

All notable changes to `audio-device-sink` will be documented in this file.

## 0.1.0

- Added the backend-neutral Rust audio sink contract.
- Added `PcmFormat` and `PcmPlaybackBuffer` for mono signed 16-bit PCM.
- Added `AudioSink`, `PlaybackReport`, `AudioSinkError`, and `NoopAudioSink`.
- Added safety guardrails for sample rate, format, and blocking playback size.

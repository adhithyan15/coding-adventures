# Changelog

All notable changes to `audio-device-coreaudio` will be documented in this file.

## 0.1.0

- Added the first macOS Core Audio implementation of `AudioSink`.
- Added blocking playback for mono signed 16-bit PCM buffers.
- Added empty-buffer no-op handling that does not require an audio device.
- Added non-macOS unsupported-platform behavior for portable CI.

# Changelog

## [0.1.0] - 2026-04-22

### Added

- Browser Web Audio sink for mono signed 16-bit PCM buffers.
- PCM validation, PCM-to-float conversion, `AudioBuffer` creation, and
  promise-based playback scheduling.
- Fake-`AudioContext` tests so CI can validate the browser sink without a real
  audio device.

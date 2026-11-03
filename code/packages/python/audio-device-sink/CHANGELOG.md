# Changelog

All notable changes to `coding-adventures-audio-device-sink` will be documented
in this file.

## [Unreleased]

### Fixed

- `BUILD_windows` stages the extension with the new
  `scripts/check_staged_extension.py` (the shape `window-native` uses) instead
  of sh one-liners (`$(...)`, `for ...; do`, `[ -f ... ]`) that `cmd /C`
  cannot run. The script stages into a fresh `mkdtemp` directory, runs ruff
  (now over `scripts` too) and pytest against it, and removes it.

## 0.1.0

- Added a Rust-backed Python audio device sink package.
- Added `play_pcm_buffer` for adapting existing `pcm_audio.PCMBuffer` objects.
- Added `play_samples` for raw mono signed 16-bit PCM playback.
- Added wrapper validation, `PlaybackReport`, and `AudioDeviceError`.
- Added a native extension that delegates playback to the Rust Core Audio sink.

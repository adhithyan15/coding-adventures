# Changelog

## 0.1.0

- Added the first text score parser for monophonic note/rest melodies.
- Added score rendering that stitches note PCM buffers and rest silence into one
  reusable `PCMBuffer`.
- Added `instrument:` and `program:` directives that render notes through the
  `musical-instruments` package.
- Added a Happy Birthday fixture that demonstrates a complete text-to-audio
  score.
- Added lazy playback helpers that delegate to `audio-device-sink` only when
  playback is requested.
- Added parser resource limits for score size, line length, and event count.
- Added `music-machine-score/v2` parsing, rendering, and playback helpers for
  explicit multi-track scores with tempo maps, instruments, chords, overlapping
  events, and safe PCM mixing.

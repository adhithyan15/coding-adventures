# Changelog

## Unreleased

- The shared Tier-0 golden (`specs/fixtures/music-machine/expected_render.json`) was regenerated once, for all three lanes together, after the Python, Rust and TypeScript `trig` ports converged on TRIG-REF-1 (PHY00a, TC-2): new `pcm16_le_sha256` `8b578c82...c730aa` (was `15c793ee...364766`); 33 of 96,138 samples moved by 1 LSB; the first and last 32 samples are unchanged. `test_tier0_golden_fixtures` is unchanged (it reads the fixture).

## 0.2.0

- Completely eliminated `import re` across `music_machine` and `score` (CWE-1333) in favor of deterministic linear-time character/string scanning.
- Integrated `percussion-kit` (`MUS07`): added `kind=drumkit` support, drumkit declarations, and `hit` events across `TextScore`, `PortableScore`, and audio renderers with multi-hit 15 ms hi-hat choke attenuation.
- Added inline `kind=additive` synthesis (`MUS05` §9) with custom harmonic partial lists and ADSR envelopes rendered via `musical-instruments`.
- Implemented deterministic canonical JSON serialization (`to_json` / `from_json`, `to_dict` / `from_dict`) on `PortableScore`, rejecting non-finite (`NaN`, `±Inf`) numbers and malformed/oversized JSON.
- Hardened defensive resource limits (CWE-400 / CWE-190 / CWE-20): enforced `MAX_SCORE_BYTES`, `MAX_EVENTS`, `MAX_INSTRUMENTS`, `MAX_TRACKS`, `MAX_SAMPLE_RATE_HZ`, `MAX_SAMPLE_COUNT`, and `MAX_DURATION_SECONDS` before buffer allocations.
- Added file path loaders `parse_score_file` and `parse_portable_score_file` with CWE-22 path validation against null bytes and traversal.
- Created Tier-0 golden fixtures in `code/specs/fixtures/music-machine/` for cross-language parity testing in Rust and TypeScript.
- Added SplitMix64 property-based testing and AST security audits.
- Added formal warrant grammar artifacts (`music_machine.warrant` and `code/specs/music-machine.warrant`).

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

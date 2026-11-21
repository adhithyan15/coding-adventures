# Changelog

## Unreleased

- VER15 CWE/CVE security hardening (MUS-W8-D):
  - **CWE-20 / CWE-190 / CWE-22**: Added O(1) `_safe_repr` error formatting across `music_machine` and `score`; caught `OverflowError` on `float(...)` (`10**400`, `Fraction(10**400, 3)`) and `_TempoMap` tick/time math into `ValueError` / `ScoreParseError`; rejected subnormal/fractional sample rates (`< 1` after rounding in `score.py`) and non-`str` identifiers/titles/meters/notes/hit_ids; added `MusicalEvent.__post_init__` validation; fixed `drumkit:` directive and `TextScore.drumkit` validation via `_valid_drum_kit_ids`; bounded `transpose_semitones` (`MAX_TRANSPOSE_SEMITONES = 1200`) and `velocity_scale` mappings (`_unit_float`, `MAX_TRACKS`); hardened `_validate_file_path` and `parse_score_file`/`parse_portable_score_file` against traversal (`..`), ADS (`:`), Windows reserved devices, ancestor symlinks/junctions, and TOCTOU/virtual-stream unbounded reads via `Path.open("rb")` + `os.fstat`.
  - **CWE-400 / CWE-674**: Added strictly iterative `_check_json_depth` (`MAX_JSON_DEPTH = 32`) pre-flight check in `PortableScore.from_json`; added `MAX_LINE_COUNT`, `MAX_POLYPHONY`, and `MAX_REPEAT_COUNT` caps; hardened `Score`, `TextScore`, `PortableScore`, `RenderedScore`, `RenderedPortableScore`, `InstrumentDeclaration`, `PortableScoreEvent`, `PhraseMotifEvent`, `ArrangementSectionEvent`, and `PortableScoreBuilder` (including `capture_section` `track_ids` and `rest`/`hit` empty-note checks) against generators, infinite iterators, and lying `__len__` (`OverflowError`) before materialization/allocation; bounded `parse_score`/`parse_portable_score` byte/line/event limits before `.encode()` and `splitlines()`.
  - Added dedicated `tests/test_hostile_inputs.py` and `tests/test_pbt.py` (`coding_adventures_test_bootstrap.prng.SplitMix64`) and `-p no:platformdirs` in `pyproject.toml`.
- Reject duplicate decoded JSON object member names in `PortableScore.from_json`, including nested and escape-equivalent duplicates.
- The shared Tier-0 golden (`specs/fixtures/music-machine/expected_render.json`) was regenerated once, for all three lanes together, after the Python, Rust and TypeScript `trig` ports converged on TRIG-REF-1 (PHY00a, TC-2): new `pcm16_le_sha256` `25bc4eb1...4acee1` (was `15c793ee...364766`); a few dozen of 96,138 samples moved by 1 LSB; the first and last 32 samples are unchanged. `test_tier0_golden_fixtures` is unchanged (it reads the fixture).

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

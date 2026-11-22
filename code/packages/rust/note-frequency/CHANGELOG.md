# Changelog

## 0.2.0

- Added `#![forbid(unsafe_code)]` and local `note-frequency.warrant` (VER15).
- Replaced unbounded `chars.collect::<String>()` with an O(N) zero-allocation ASCII scanner enforcing `MAX_NOTE_STRING_LENGTH = 32` prior to parsing (**CWE-400**).
- Fixed `i32` overflow in `Note::semitones_from_a4` via `i64` arithmetic, added `checked_semitones_from_a4()`, and enforced octave bounds `[-100, 100]` (**CWE-190**).
- Added `validate_frequency()`, `validate_reference_frequency()`, `checked_frequency()`, and `note_to_frequency_with_reference()` rejecting `NaN`, `±Inf`, and out-of-range frequency inputs (**CWE-20**).
- Added `tests/test_hostile_inputs.rs` and `tests/test_pbt.rs` (SplitMix64 PRNG).

## 0.1.0

- Added the initial note-to-frequency package.
- Added note parsing for natural notes plus single sharps and flats.
- Added equal-tempered frequency mapping anchored at `A4 = 440 Hz`.
- Added tests that lock down the shared parity vectors across language ports.

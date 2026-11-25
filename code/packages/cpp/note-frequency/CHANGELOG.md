# Changelog

All notable changes to the C++ `note-frequency` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] - 2026-11-20

### Added

- Added `required_capabilities.json` (`"capabilities": []`) and local
  `note-frequency.warrant` aligned with `code/specs/note-frequency.warrant`.
- Added `MAX_NOTE_STRING_LENGTH` (`32`), `MIN_OCTAVE` (`-100`), `MAX_OCTAVE`
  (`100`), `MAX_FREQUENCY_HZ` (`1e38`), and `MAX_REFERENCE_FREQUENCY_HZ`
  (`1e6`).
- Added `validate_frequency`, `Note::frequency_with_reference`,
  `note_to_frequency_with_reference`, and `const char*` overloads for
  `Note::Note`, `parse_note`, `note_to_frequency`, and
  `note_to_frequency_with_reference`.
- Added hostile/adversarial CWE-20/190/400/476 tests and SplitMix64 PBT/fuzz
  tests in `tests/note_frequency_test.cpp`.

### Security & Fixes

- **CWE-476**: Guarded `const char*` overloads on `Note::Note`, `parse_note`,
  `note_to_frequency`, and `note_to_frequency_with_reference` against `nullptr`
  prior to `std::string` construction.
- **CWE-400**: Enforced `MAX_NOTE_STRING_LENGTH` (32) and 1-char `letter`/
  `accidental` bounds prior to `std::string` allocation or error formatting.
- **CWE-20 / CWE-190**: Rejected multi-character `letter` inputs and embedded
  NUL accidentals, bounded `octave` to `[-100, 100]` with overflow-free
  decimal parsing, fixed `pow2i(INT_MIN)` signed negation UB using unsigned
  magnitude arithmetic, computed semitones in `std::int64_t`, and rejected
  non-finite (`NaN`/`±Inf`) and out-of-range frequencies without `<cmath>`.

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `note-frequency` crate
  (namespace `ca::note_frequency`).
- A `Note` value class (`spelling` / `chromatic_index` / `semitones_from_a4` /
  `frequency` / `to_string` / `operator==`) plus free functions `parse_note`
  and `note_to_frequency`.
- 12-TET frequency `440 * 2^(semitones_from_A4 / 12)`, with `2^x` computed from
  a libm-free `e^x` (Cody-Waite range reduction + Taylor series).
- Spelling validation against the real chromatic pitches (rejects
  `Cb`/`E#`/`B#`/`Fb`); parsing accepts an optional `#`/`b` accidental and a
  signed octave. Malformed input throws `std::invalid_argument` in place of the
  Rust `Result<_, String>`.
- Semitone arithmetic is done in 64-bit and `e^x` bounds its argument, so even
  an extreme (crafted) octave produces a defined saturated frequency rather than
  signed-overflow / float-to-int UB.
- 34 checks over reference pitches, enharmonics, semitone distances, parse
  errors, and extreme octaves, run under every available C++ compiler via the
  shared `iso-harness`.

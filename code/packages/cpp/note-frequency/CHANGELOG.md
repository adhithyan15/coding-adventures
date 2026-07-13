# Changelog

All notable changes to the C++ `note-frequency` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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

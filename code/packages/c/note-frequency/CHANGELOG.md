# Changelog

All notable changes to the C `note-frequency` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.2.0] - 2026-11-20

### Added

- Added `required_capabilities.json` (`"capabilities": []`) and local
  `note-frequency.warrant` aligned with `code/specs/note-frequency.warrant`.
- Added `NF_MAX_NOTE_STRING_LENGTH` (`32`), `NF_MIN_OCTAVE` (`-100`),
  `NF_MAX_OCTAVE` (`100`), `NF_MAX_FREQUENCY_HZ` (`1e38`), and
  `NF_MAX_REFERENCE_FREQUENCY_HZ` (`1e6`).
- Added `NF_ERR_NULL_POINTER` and `NF_ERR_INVALID_FREQUENCY` status codes.
- Added `nf_validate_frequency`, `nf_note_frequency_with_reference`,
  `nf_note_to_frequency_with_reference`, `nf_note_checked_semitones_from_a4`,
  and `nf_note_checked_frequency` APIs with `<math.h>`-free finite frequency
  validation.
- Added hostile/adversarial CWE-20/190/400/476 tests and SplitMix64 PBT/fuzz
  tests in `tests/note_frequency_test.c`.

### Security & Fixes

- **CWE-476**: Guarded all public functions against `NULL` pointers,
  zero-length/tiny output buffers, and non-NUL-terminated `NfNote.accidental`
  buffers.
- **CWE-400**: Enforced `NF_MAX_NOTE_STRING_LENGTH` (32) via bounded
  `bounded_strlen` prior to parsing and bounded `letter`/`accidental` reads.
- **CWE-20 / CWE-190**: Rejected multi-character `letter` inputs instead of
  silently truncating, bounded `octave` to `[-100, 100]` with overflow-free
  decimal parsing, fixed `pow2i(INT_MIN)` signed negation UB using unsigned
  magnitude arithmetic, and computed semitones in `int64_t`.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `note-frequency` crate.
- `NfNote` value type plus `nf_note_new`, `nf_parse_note`,
  `nf_note_to_frequency`, and the accessors `nf_note_spelling` /
  `nf_note_chromatic_index` / `nf_note_semitones_from_a4` / `nf_note_frequency`
  / `nf_note_to_string`.
- 12-TET frequency `440 * 2^(semitones_from_A4 / 12)`, with `2^x` computed from
  a libm-free `e^x` (Cody-Waite range reduction + Taylor series).
- Spelling validation against the real chromatic pitches (rejects
  `Cb`/`E#`/`B#`/`Fb`); note parsing accepting an optional `#`/`b` accidental and
  a signed octave.
- `NfStatus` status-code API (`NF_OK` / `NF_ERR_INVALID_SPELLING` /
  `NF_ERR_INVALID_NOTE`) in place of the Rust `Result<_, String>`.
- Semitone arithmetic is done in 64-bit and `e^x` bounds its argument, so even
  an extreme (crafted) octave produces a defined saturated frequency rather than
  signed-overflow / float-to-int UB.
- 52 checks over reference pitches, enharmonics, semitone distances, parse
  errors, and extreme octaves, run under every available C compiler via the
  shared `iso-harness`.

# Changelog

All notable changes to the C `note-frequency` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

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

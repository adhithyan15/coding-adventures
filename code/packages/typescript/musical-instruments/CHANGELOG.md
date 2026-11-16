# Changelog

## Unreleased

### Fixed
- `musical-instruments` Warrant document fixed: `| guard.` claims put on one line, invalid `total_samples` bound replaced by the existing claim, ops with `rejects` now return Result. Document only.

## 0.1.0

- Initial TypeScript implementation of MUS05 Naive Instrument Models and Timbre.
- Additive harmonic profile synthesis and ADSR loudness envelope shaping.
- 128 General MIDI program catalog and 21 built-in naive instrument presets.
- Deterministic SplitMix64 seeded micro-variation profile.
- VER15 CWE-20, CWE-190, CWE-400, CWE-674, CWE-1333 proofs and verification suites.

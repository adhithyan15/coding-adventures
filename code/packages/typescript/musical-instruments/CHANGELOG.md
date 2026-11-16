# Changelog

## Unreleased

### Fixed
- `musical-instruments` Warrant document fixed: `| guard.` claims put on one line, invalid `total_samples` bound replaced by the existing claim, ops with `rejects` now return Result. Document only. Note: the `instrument_signal_sample` bound `steps <= 64 * partial_count + 128` became `64 * 64 + 128` (`partial_count` is not that operation's parameter); this loosens it to the worst case and is sound because `MAX_PARTIALS = 64`.

## 0.1.0

- Initial TypeScript implementation of MUS05 Naive Instrument Models and Timbre.
- Additive harmonic profile synthesis and ADSR loudness envelope shaping.
- 128 General MIDI program catalog and 21 built-in naive instrument presets.
- Deterministic SplitMix64 seeded micro-variation profile.
- VER15 CWE-20, CWE-190, CWE-400, CWE-674, CWE-1333 proofs and verification suites.

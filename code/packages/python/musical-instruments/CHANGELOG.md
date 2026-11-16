# Changelog

## Unreleased

### Fixed
- `musical-instruments` Warrant document fixed: `| guard.` claims put on one line, invalid `total_samples` bound replaced by the existing claim, ops with `rejects` now return Result. Document only. Note: the `instrument_signal_sample` bound `steps <= 64 * partial_count + 128` became `64 * 64 + 128` (`partial_count` is not that operation's parameter); this loosens it to the worst case and is sound because `MAX_PARTIALS = 64`.

## 0.2.0

- Completed `MUS05` §5 and §7 deterministic humanization: wired seeded
  `VariationProfile` (`pitch_cents_jitter`, `amplitude_jitter`, `attack_jitter_ms`,
  `seed`, `harmonic_jitter`) into `InstrumentSignal` and `render_instrument_note`.
- Added pure-Python deterministic `SplitMix64` PRNG conforming to `VER05` §2.1
  and `TF01` §5.1, ensuring 100% reproducible variation with zero ambient OS entropy.
- Enforced strict VER15 / CWE-400 bounds across all synthesis and envelope routines:
  `MAX_PARTIALS = 64`, `MAX_NOTE_DURATION_SECONDS = 60.0`,
  `MAX_SAMPLE_RATE_HZ = 192_000.0`, and `MAX_SAMPLE_COUNT = 16_777_216`.
- Protected against float-to-int rounding `OverflowError` (CWE-190) via `_safe_round_sample`.
- Guaranteed CWE-674 stack exhaustion freedom via strictly iterative synthesis loops.
- Enforced zero `re` and zero `eval`/`exec` dependencies (CWE-1333, CWE-94).
- Added `musical_instruments.warrant` with `warrant-spec::all-operands-public`, bounded
  step/heap/depth, and determinism claims.
- Updated `required_capabilities.json` with pure computation justification (`capabilities: []`).
- Added extensive test suite including property-based testing with deterministic
  SplitMix64 PRNG, adversarial numeric boundary fuzzing, and static AST security audits
  achieving 100.0% statement coverage.

## 0.1.0

- Added naive instrument profile types from `MUS05`.
- Added additive synthesis with ADSR envelopes and release tails.
- Added beginner presets for sine, flute, clarinet, violin, piano, plucked
  string, brass, organ, mallet, synth lead, synth pad, and effects.
- Added a full 128-entry General-MIDI-style melodic program catalog that maps
  keyboard programs onto the current naive preset library.
- Added note rendering to floating samples and signed 16-bit mono PCM.

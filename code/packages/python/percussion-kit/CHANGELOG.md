# Changelog

## 0.2.0

- Completed `MUS07` §6 and §11 starter kit: added all 5 remaining starter voices
  (`clap`, `cowbell`, `tambourine`, `shaker`, and `woodblock`) to `standard_kit`,
  bringing the portable starter kit to all 15 specified voices.
- Added sequence rendering and choke-group dynamics (`render_percussion_sequence`,
  `PercussionHitEvent`, `PercussionSequenceRender`, `apply_hihat_choke`) per `MUS07`
  §7 & §9, where `closed_hihat` or `pedal_hihat` attenuates active `open_hihat`
  with a 15 ms linear release ramp.
- Enforced strict VER15 / CWE-400 bounds across all synthesis and sequence routines:
  `MAX_PERCUSSION_MODES = 32`, `MAX_HIT_DURATION_SECONDS = 30.0`,
  `MAX_SAMPLE_COUNT = 16_777_216`, and `MAX_SEQUENCE_EVENTS = 16_384`.
- Guaranteed CWE-674 stack exhaustion freedom via strictly iterative synthesis loops.
- Enforced zero `re` and zero `eval`/`exec` dependencies (CWE-1333, CWE-94).
- Added `percussion_kit.warrant` with `all-operands-public`, bounded step/heap/depth,
  and determinism claims.
- Updated `required_capabilities.json` with pure computation justification (`capabilities: []`).
- Added extensive test suite including property-based testing with deterministic
  SplitMix64 PRNG, adversarial numeric boundary fuzzing, and static AST security audits
  achieving 100.0% statement coverage.

## 0.1.0

- Added a deterministic naive unpitched percussion package for drum-kit style
  hits.
- Added `PercussionVoiceProfile`, `DrumKitProfile`, and `PercussionSignal`.
- Added a built-in `standard_kit` with kick, snare, hi-hat, tom, crash, and
  ride voices.
- Added inspectable hit rendering to floating samples and signed 16-bit mono
  PCM.

# Changelog

## 0.3.1 - 2026-11-21

### Hardened
- VER15 CWE/CVE security hardening (MUS-W8-D):
  - **CWE-20 / CWE-190**: Added O(1) `_safe_repr` error formatting across numeric, kit, voice, and sequence validation; caught `OverflowError` on `float(...)` (`10**400`, `Fraction(10**400, 3)`) into `ValueError`; rejected subnormal sample rates (`not isfinite(1.0 / rate)`) and overflowing voice component amplitude sums (`not isfinite(total)`); added strict `str`/`Mapping` checks on `PercussionVoiceProfile` / `DrumKitProfile` / `PercussionHitEvent`; added stage-field `isinstance` checks in `PercussionHitRender.__post_init__` and `PercussionSequenceRender.__post_init__`; turned missing `hit_id` in dict events from `KeyError` into `ValueError`.
  - **CWE-400**: Hardened `PercussionVoiceProfile.modes`, `apply_hihat_choke`, and `render_percussion_sequence` against generators, infinite iterators, non-iterables (`str`/`bytes`/`Mapping`), and lying `__len__` (`OverflowError` or exceeding cap) before buffer allocation or materialization.
  - Added dedicated `tests/test_hostile_inputs.py` and `tests/test_pbt.py` (`coding_adventures_test_bootstrap.prng.SplitMix64`) and `-p no:platformdirs` in `pyproject.toml`.

## 0.3.0 - 2026-11-16

### Changed
- A non-finite phase (e.g. `frequency * time` overflowing to inf) now yields NaN instead of raising `OverflowError` from `math.floor`, matching the Rust and TypeScript lanes. Finite inputs are bit-identical (the shared music-machine golden is unchanged).

### Changed
- **Fix (review of TRIG-CONV TC-2):** the pseudo-noise phase `(i + 1) * (seed * 0.1618 + 0.618)` is unbounded, and `trig.sin` is NaN beyond 2^29 radians (PHY00a 3.4.1), so the noise failed at sample 0 for seed 1e9, after about 43,000 samples for seed 12345 and at 29.6 s at 192 kHz for seed 89. The phase is now reduced to its fractional cycle (`phase - floor(phase)`, exact) before the `2*pi` multiply, identically in Python, Rust and TypeScript; the mode sines are reduced the same way. Noise samples change in the last bits. Tests at the failing seeds and counts, with the same expected values in all three lanes.

### Fixed
- `percussion-kit` Warrant document fixed: `| guard.` claims put on one line, invalid `total_samples` bound replaced by the existing claim, ops with `rejects` now return Result. Document only.

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

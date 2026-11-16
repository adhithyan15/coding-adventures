# Changelog

## 0.2.0 - 2026-11-16

### Changed
- Mode and noise sines come from the bit-exact `trig` package (TRIG-REF-1, PHY00a) instead of `Math.sin`: byte-identical to the Python and Rust lanes. New dependency `trig` (`file:../trig`).

## Unreleased

### Fixed
- `percussion-kit` Warrant document fixed: `| guard.` claims put on one line, invalid `total_samples` bound replaced by the existing claim, ops with `rejects` now return Result. Document only.

## 0.1.0

- Initial TypeScript implementation of MUS07 Unpitched Percussion and Drum Kit Events.
- Standard 15-voice starter drum kit and canonical alias resolution.
- Resonant modes, deterministic decaying noise bursts, and attack-hold-decay envelopes.
- Single hit rendering and timeline sequence rendering with hi-hat choke dynamics (15 ms linear ramp).
- VER15 CWE-20, CWE-190, CWE-400, CWE-674, CWE-1333 proofs and verification suites.

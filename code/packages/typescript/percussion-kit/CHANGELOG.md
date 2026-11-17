# Changelog

## 0.2.0 - 2026-11-16

### Changed
-- **Fix (review of TRIG-CONV TC-2):** the pseudo-noise phase `(i + 1) * (seed * 0.1618 + 0.618)` is unbounded, and `trig.sin` is NaN beyond 2^29 radians (PHY00a 3.4.1), so the noise failed at sample 0 for seed 1e9, after about 43,000 samples for seed 12345 and at 29.6 s at 192 kHz for seed 89. The phase is now reduced to its fractional cycle (`phase - floor(phase)`, exact) before the `2*pi` multiply, identically in Python, Rust and TypeScript; the mode sines are reduced the same way. Noise samples change in the last bits. Tests at the failing seeds and counts, with the same expected values in all three lanes.
 Mode and noise sines come from the bit-exact `trig` package (TRIG-REF-1, PHY00a) instead of `Math.sin`: byte-identical to the Python and Rust lanes. New dependency `trig` (`file:../trig`).

## Unreleased

### Fixed
- `percussion-kit` Warrant document fixed: `| guard.` claims put on one line, invalid `total_samples` bound replaced by the existing claim, ops with `rejects` now return Result. Document only.

## 0.1.0

- Initial TypeScript implementation of MUS07 Unpitched Percussion and Drum Kit Events.
- Standard 15-voice starter drum kit and canonical alias resolution.
- Resonant modes, deterministic decaying noise bursts, and attack-hold-decay envelopes.
- Single hit rendering and timeline sequence rendering with hi-hat choke dynamics (15 ms linear ramp).
- VER15 CWE-20, CWE-190, CWE-400, CWE-674, CWE-1333 proofs and verification suites.

# Changelog

All notable changes to `sim-discrete-audio-timer` will be documented in this file.

## [0.1.0] - 2026-11-15

### Added
- Initial implementation of `sim-discrete-audio-timer` (GAME15c).
- `Timer555Astable`: Astable multivibrator model with cycle-accurate $R_1, R_2, C$ analog square-wave synthesis.
- Pong audio presets: paddle deflection (~480 Hz), wall collision (~240 Hz), score (~120 Hz / 960 Hz).
- `DiscreteNoiseFilter`: 17-bit and 15-bit shift-register LFSR noise generator with RC low-pass filter and diode shaping for explosions.
- `CoinMechDebounce`: Mechanical coin acceptor switch debounce and credit latching with `logic-gates` SR-latch integration and saturating credit counter (CWE-190).
- Zero-allocation audio rendering into PCM slices, `PcmRingBuffer`, and `PcmAudio`.
- Level 5 Microbenchmark suite with counting allocator verification and sabotage detection (VER07).
- Runtime capability verification harness (VER08b).

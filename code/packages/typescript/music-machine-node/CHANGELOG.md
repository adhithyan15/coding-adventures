# Changelog

## Unreleased

- The shared Tier-0 golden (`specs/fixtures/music-machine/expected_render.json`) was regenerated once, for all three lanes together, after the Python, Rust and TypeScript `trig` ports converged on TRIG-REF-1 (PHY00a, TC-2): new `pcm16_le_sha256` `25bc4eb1...4acee1` (was `15c793ee...364766`); a few dozen of 96,138 samples moved by 1 LSB; the first and last 32 samples are unchanged. New `tests/golden_parity.test.ts` asserts the digest and the first/last 32 samples (the lane had no golden test). Depends on `trig` through `oscillator`, `musical-instruments` and `percussion-kit`; BUILD files build it first.

## 0.1.0

- Initial release of `@coding-adventures/music-machine-node`.
- Sheet music text score parser with linear-time directive and token scanners.
- Canonical `music-machine-score/v2` multi-track parser, builder, motif capturer, and arranger.
- Deterministic JSON score interchange with depth bounding and prototype pollution prevention.
- Additive synthesizer and percussion kit sequence rendering with hi-hat choke dynamics.
- Audio device sink playback integration with `@coding-adventures/audio-device-sink`.
- Full VER15 proofs against CWE-20, CWE-190, CWE-400, CWE-674, CWE-1321, and CWE-1333.

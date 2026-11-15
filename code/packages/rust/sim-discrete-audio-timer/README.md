# sim-discrete-audio-timer (GAME15c)

Deterministic 555-timer, LFSR noise, and analog filter simulation for coin-op arcade audio.

## Architecture

This crate provides discrete component simulation for 1970s coin-operated arcade machines:
1. **`Timer555Astable`**: Astable multivibrator modeling discrete $R_1, R_2, C$ analog square-wave oscillators for Pong paddle hit (~480 Hz), wall bounce (~240 Hz), and score (~120 Hz / 960 Hz) tones.
2. **`DiscreteNoiseFilter`**: 17-bit and 15-bit shift-register pseudo-random noise generator paired with an RC single-pole low-pass filter and diode clipping/shaping stage for arcade explosions and thuds.
3. **`CoinMechDebounce`**: Mechanical coin acceptor switch debounce and credit latching logic with contact glitch rejection, `logic-gates` SR-latch integration, and saturating credit counter.
4. **Zero-Allocation PCM Rendering**: Stream direct 16-bit audio into slices, `audio-mixer-realtime::PcmRingBuffer`, and `pcm-audio::PcmAudio`.

## Verification & Security Guarantees

- **Safety**: `#![forbid(unsafe_code)]` throughout the entire crate.
- **Pure Computation**: Declares zero ambient capabilities (`{"capabilities": []}` in `required_capabilities.json`).
- **CWE-369 (Divide by Zero)**: Guarded against zero resistances, zero capacitance, and invalid sample rates.
- **CWE-190 (Integer Overflow)**: Polyphonic mixing accumulation in 64-bit with saturating clamp to `[-32768, 32767]`; credit counter saturation.
- **CWE-400 (Resource Exhaustion)**: Hard zero dynamic allocation during real-time sample stepping and buffer rendering.

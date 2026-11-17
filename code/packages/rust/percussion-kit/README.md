# percussion-kit

Deterministic unpitched percussion voices, standard drum kit, and choke sequences in safe Rust (MUS07).

Implements the unpitched percussion model:

```text
percussion hit + kit voice profile -> transient + noise + resonances -> shaped waveform -> PCM audio
```

## Features

- **All 15 Portable Starter Kit Voices**: `kick`, `snare`, `closed_hihat`, `open_hihat`, `pedal_hihat`, `low_tom`, `mid_tom`, `high_tom`, `crash`, `ride`, `rimshot`, `clap`, `cowbell`, `tambourine`, `shaker`, `woodblock`.
- **Canonical Alias Resolution**: Resolves drummer / score shorthands (`bd`, `sd`, `ch`, `oh`, `ph`, `lt`, `mt`, `ht`, `cc`, `rc`, `rim`, `side_stick`, `cp`, `cb`, `tb`, `sh`, `wb`).
- **Deterministic Noise Synthesis**: Deterministic pseudo-noise and `SplitMix64` PRNG noise synthesis with zero ambient entropy or external RNG state.
- **Hi-Hat Choke Group Dynamics**: 15 ms linear release ramp muting of active open hi-hats when struck by closed, pedal, or repeated open hi-hat hits.
- **Streaming Sequence Rendering**: Bounded memory synthesis mixing events iteratively into the timeline without buffering separate tracks.

## Security & Verification Guarantees

- **Safety**: `#![forbid(unsafe_code)]`, zero third-party crate dependencies (only `std`, `oscillator`, and `pcm-audio`).
- **Strictly pure capabilities**: `[]` declared in `required_capabilities.json`.
- **CWE-400 (Resource Exhaustion)**:
  - `MAX_PERCUSSION_MODES = 32`
  - `MAX_DURATION_SECONDS = 60.0`
  - `MAX_SAMPLE_RATE_HZ = 192_000.0`
  - `MAX_SAMPLE_COUNT = 16_777_216` (2^24)
  - `MAX_HITS = 65_536`
  - Pre-flight budget verification strictly before buffer allocation.
- **CWE-190 (Integer Overflow)**: Safe rounding and checked arithmetic on sample counts and timeline indices.
- **CWE-20 (Improper Input Validation)**: Finite-float checks rejecting `NaN` and `±Inf`.
- **CWE-674 (Uncontrolled Recursion)**: Strictly iterative synthesis routines with provably constant stack depth.
- **Verification Warrant**: `percussion-kit.warrant` and `code/specs/percussion-kit.warrant`.

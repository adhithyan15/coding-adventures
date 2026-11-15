# @coding-adventures/musical-instruments

`@coding-adventures/musical-instruments` is the deterministic TypeScript implementation of `MUS05: Naive Instrument Models and Timbre`.

It adds the missing box between a note's frequency and the sampled audio:

```text
note frequency + instrument profile -> shaped waveform -> sampler -> PCM
```

This package models instruments as inspectable additive synthesis recipes with deterministic humanization:

- a harmonic profile specifies which sine waves are mixed together
- an ADSR envelope shapes loudness over the note's lifetime and release tail
- a deterministic seeded `VariationProfile` introduces micro-variations (pitch, amplitude, attack timing, overtones)
- a pure SplitMix64 PRNG drives variations with zero ambient entropy
- strict CWE-400 / CWE-190 / CWE-20 / CWE-674 / CWE-1333 resource and recursion bounds are enforced
- formal VER verification artifacts (`musical-instruments.warrant`, `required_capabilities.json`)

## Keyboard-Style Catalog

The package exposes a full 128-entry General-MIDI-style melodic program catalog.

## Invariants & Guarantees (VER15)

- **CWE-400 (Resource Exhaustion)**: `MAX_PARTIALS = 64`, `MAX_NOTE_DURATION_SECONDS = 60.0`, `MAX_SAMPLE_RATE_HZ = 192_000.0`, `MAX_SAMPLE_COUNT = 16_777_216`.
- **CWE-190 (Integer Overflow)**: Safe rounding and checked safe integers on all sample counts.
- **CWE-20 (Improper Input Validation)**: Finite float checks on all numeric inputs; `NaN` and `±Inf` fail closed.
- **CWE-674 (Uncontrolled Recursion)**: Strictly iterative synthesis routines with constant call stack depth.
- **CWE-1333 / CWE-94**: Zero regular expressions and zero dynamic eval.
- **Pure Computation**: `capabilities: []` declared in `required_capabilities.json`.

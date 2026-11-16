# @coding-adventures/percussion-kit

`@coding-adventures/percussion-kit` is the deterministic TypeScript implementation of `MUS07: Unpitched Percussion and Drum Kit Events`.

It models percussion in two tiers:

```text
single hit:
  hit id + kit voice profile -> transient + noise + resonances -> shaped waveform -> sampled PCM

sequence:
  percussion events + choke groups -> timeline mixing + release ramp -> sampled PCM
```

This package is deliberately deterministic and beginner-friendly:

- all 15 `MUS07` starter voices in `standardKit()`
- single-hit inspectable rendering with mode resonance and decaying noise
- multi-event sequence rendering with hi-hat choke group dynamics (15 ms release ramp)
- strict CWE-400 / CWE-190 / CWE-20 / CWE-674 / CWE-1333 resource and recursion bounds
- formal VER verification artifacts (`percussion-kit.warrant`, `required_capabilities.json`)

## Standard Kit

The built-in `standardKit()` exposes all 15 portable starter hit ids from `MUS07` §6:
`kick`, `snare`, `closed_hihat`, `open_hihat`, `pedal_hihat`, `low_tom`, `mid_tom`, `high_tom`, `crash`, `ride`, `clap`, `cowbell`, `tambourine`, `shaker`, `woodblock`.

## Invariants & Guarantees (VER15)

- **CWE-400 (Resource Exhaustion)**: `MAX_PERCUSSION_MODES = 32`, `MAX_HIT_DURATION_SECONDS = 30.0`, `MAX_SAMPLE_COUNT = 16_777_216`, `MAX_SEQUENCE_EVENTS = 16_384`.
- **CWE-190 (Integer Overflow)**: Safe rounding and checked safe integers on all sample counts.
- **CWE-20 (Improper Input Validation)**: Finite float checks on all numeric inputs; `NaN` and `±Inf` fail closed.
- **CWE-674 (Uncontrolled Recursion)**: Strictly iterative synthesis and mixing routines with constant call stack depth.
- **CWE-1333 / CWE-94**: Zero regular expressions and zero dynamic eval.
- **Pure Computation**: `capabilities: []` declared in `required_capabilities.json`.

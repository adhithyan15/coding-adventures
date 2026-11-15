# percussion-kit

`percussion-kit` is the deterministic implementation of `MUS07: Unpitched Percussion
and Drum Kit Events`.

It models percussion in two tiers:

```text
single hit:
  hit id + kit voice profile -> transient + noise + resonances -> sampled PCM

sequence:
  percussion events + choke groups -> timeline mixing + release ramp -> sampled PCM
```

This package is deliberately deterministic and beginner-friendly. It provides:

- all 15 `MUS07` starter voices in `standard_kit`
- single-hit inspectable rendering with mode resonance and decaying noise
- multi-event sequence rendering with hi-hat choke group dynamics (15 ms release ramp)
- strict CWE-400 / CWE-674 / CWE-1333 resource and recursion bounds
- formal VER verification artifacts (`percussion_kit.warrant`, `required_capabilities.json`)

## Standard Kit

The built-in `standard_kit` exposes all 15 portable starter hit ids from `MUS07` §6:

- `kick` — bass drum / kick
- `snare` — snare drum
- `closed_hihat` — closed hi-hat (choke group `hihat`)
- `open_hihat` — open hi-hat (choke group `hihat`)
- `pedal_hihat` — foot / pedal hi-hat (choke group `hihat`)
- `low_tom` — low tom
- `mid_tom` — middle tom
- `high_tom` — high tom
- `crash` — crash cymbal
- `ride` — ride cymbal
- `clap` — hand clap
- `cowbell` — tuned metallic cowbell
- `tambourine` — frame tambourine with zils
- `shaker` — stochastic noise shaker
- `woodblock` — dry hollow slit woodblock

## Usage

### Single Hit Rendering

```python
from percussion_kit import render_percussion_hit

rendered = render_percussion_hit(
    "snare",
    duration_seconds=0.08,
    sample_rate_hz=44_100,
    amplitude=0.35,
)

print(rendered.voice.id)                    # snare_naive
print(rendered.floating_samples.sample_count())
print(rendered.pcm_buffer.samples[:8])
```

### Sequence Rendering with Hi-Hat Choke Dynamics

Per `MUS07` §7 and §9, when a `closed_hihat` or `pedal_hihat` occurs while an
`open_hihat` is actively ringing, the open hi-hat is smoothly attenuated using
a 15 ms linear release ramp:

```python
from percussion_kit import PercussionHitEvent, render_percussion_sequence

events = [
    PercussionHitEvent("kick", start_seconds=0.0, velocity=0.9),
    PercussionHitEvent("open_hihat", start_seconds=0.0, velocity=0.7),
    PercussionHitEvent("closed_hihat", start_seconds=0.15, velocity=0.6),
    PercussionHitEvent("snare", start_seconds=0.25, velocity=0.8),
]

rendered = render_percussion_sequence(events, sample_rate_hz=44_100)
print(f"Total duration: {rendered.duration_seconds:.3f}s")
print(f"Sample count: {rendered.sample_count()}")
```

## Security & Verification Invariants (VER15)

In accordance with the Verified Shipping Program:

- **CWE-400 (Uncontrolled Resource Consumption):** Strictly bounded by
  `MAX_PERCUSSION_MODES = 32`, `MAX_HIT_DURATION_SECONDS = 30.0`,
  `MAX_SAMPLE_COUNT = 16_777_216`, and `MAX_SEQUENCE_EVENTS = 16_384`.
- **CWE-674 (Uncontrolled Recursion):** Synthesis and mixing routines are
  strictly iterative with constant call stack depth.
- **CWE-1333 (ReDoS) / CWE-94 (Code Injection):** Zero regular expressions
  (`re`) and zero dynamic code execution (`eval`/`exec`).
- **CWE-208 (Timing Side-Channels):** All operands are public (`percussion_kit.warrant`),
  establishing the `warrant-spec::all-operands-public` derived attestation.
- **Required Capabilities:** `capabilities: []` declared in `required_capabilities.json`
  (pure computation).

## What This Is Not Yet

This package does not yet model:

- portable score `hit` event parsing (handled at the `music-machine` score layer)
- filtered noise bands (e.g. biquad bandpass cymbal filters)
- pitch-drop kick drums (frequency envelopes)
- sample-based drum soundfont libraries
- finite-element physical membrane simulation

Those belong to later layers. This package provides the portable baseline voice,
kit, and sequence abstraction.

## Development

```bash
bash BUILD
```

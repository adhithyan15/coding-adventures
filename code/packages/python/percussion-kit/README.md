# percussion-kit

`percussion-kit` is the first implementation of `MUS07: Unpitched Percussion
and Drum Kit Events`.

It models a percussion hit as:

```text
hit id + kit voice profile -> transient + noise + resonances -> sampled PCM
```

This package is deliberately deterministic and beginner-friendly. It does not
try to be a perfect sampled drum library. It gives us a stable first abstraction
for:

- drum machines
- groove tracks
- rhythm sections
- later `music-machine` `hit` events

## Standard Kit

The built-in `standard_kit` exposes these hit ids:

- `kick`
- `snare`
- `closed_hihat`
- `open_hihat`
- `pedal_hihat`
- `low_tom`
- `mid_tom`
- `high_tom`
- `crash`
- `ride`

## Usage

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

The same hit rendered twice is deterministic by default, which keeps tests and
fixtures stable.

## What This Is Not Yet

This package does not yet model:

- portable score `hit` events
- hi-hat choke interaction during timeline mixing
- filtered noise bands
- pitch-drop kick drums
- sample-based drum kits
- physical membrane or cymbal simulation

Those belong to later layers. This package just establishes the first reusable
voice and kit abstraction.

## Development

```bash
bash BUILD
```

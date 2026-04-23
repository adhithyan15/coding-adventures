# musical-instruments

`musical-instruments` is the first implementation of `MUS05: Naive Instrument
Models and Timbre`.

It adds the missing box between a note's frequency and the sampled audio:

```text
note frequency + instrument profile -> shaped waveform -> sampler -> PCM
```

V1 is deliberately naive. A flute, violin, piano, and plucked string are not
physical simulations yet. They are small additive-synthesis recipes:

- a harmonic profile says which sine waves are mixed together
- an ADSR envelope says how the loudness changes over the note's lifetime
- a gain keeps the result safe for PCM output

## Keyboard-Style Catalog

The package exposes a full 128-entry General-MIDI-style melodic program catalog.
That gives us the same kind of "program number selects a voice" idea found on
many keyboards.

The first catalog is wide, not perfect: all 128 programs are selectable, but
many programs intentionally share a smaller set of naive archetypes until we
teach each family better behavior.

Examples:

```python
from musical_instruments import (
    get_gm_program,
    instrument_for_gm_program,
    render_instrument_note,
)

program = get_gm_program(74)              # Flute
instrument = instrument_for_gm_program(74)

rendered = render_instrument_note(
    "A4",
    duration_seconds=0.5,
    instrument=instrument,
    amplitude=0.2,
)

print(program.name)                       # Flute
print(rendered.instrument.id)             # flute_naive
print(rendered.pcm_buffer.samples[:5])
```

## Built-In Naive Presets

The first named presets are:

| Id | Meaning |
|----|---------|
| `sine` | pure sine tone |
| `flute_naive` | breathy-ish light harmonic mix |
| `clarinet_naive` | odd-harmonic reed-like mix |
| `violin_naive` | richer bowed-string-like harmonic mix |
| `piano_naive` | fast attack, decaying struck-string-like tone |
| `pluck_naive` | fast attack, plucked-string-like decay |
| `brass_naive` | bright brass-like harmonic mix |
| `organ_naive` | steady drawbar-like harmonic mix |
| `mallet_naive` | percussive chromatic percussion archetype |
| `synth_lead_naive` | bright synth lead archetype |
| `synth_pad_naive` | slow-attack pad archetype |
| `effect_naive` | inharmonic placeholder for effects |

## What This Is Not Yet

This is not a Yamaha, Casio, Roland, or Korg voice clone. Real keyboard patches
often combine sampled attacks, looping sustain regions, filters, effects,
velocity layers, and proprietary tuning choices.

This package gives us a teachable foundation that can later grow into:

- better per-instrument harmonic profiles
- velocity-sensitive envelopes
- sampled attacks
- filters and vibrato
- physical models for strings, reeds, pipes, and resonating bodies

## Development

```bash
bash BUILD
```

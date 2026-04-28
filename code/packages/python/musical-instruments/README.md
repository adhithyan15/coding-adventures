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
| `celesta_naive` | soft bell-like pitched percussion |
| `glockenspiel_naive` | bright metallic pitched percussion |
| `vibraphone_naive` | sustained metallic bar pitched percussion |
| `marimba_naive` | warm woody bar pitched percussion |
| `xylophone_naive` | short bright wooden bar pitched percussion |
| `tubular_bells_naive` | long-ringing bell-like pitched percussion |
| `timpani_naive` | drum-like but still note-pitched percussion |
| `kalimba_naive` | plucked tine pitched percussion |
| `synth_lead_naive` | bright synth lead archetype |
| `synth_pad_naive` | slow-attack pad archetype |
| `effect_naive` | inharmonic placeholder for effects |

These are still rendered through the same note-based path as melodic
instruments:

```text
note -> frequency -> instrument signal -> sampled PCM
```

That is the key distinction for this phase: pitched percussion still has a
stable note/frequency center, even though its overtone mix and envelope feel
more like a struck bar, bell, tine, or drum shell than a flute or violin.

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

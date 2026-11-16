# musical-instruments

`musical-instruments` is the deterministic implementation of `MUS05: Naive Instrument
Models and Timbre`.

It adds the missing box between a note's frequency and the sampled audio:

```text
note frequency + instrument profile -> shaped waveform -> sampler -> PCM
```

This package models instruments as inspectable additive synthesis recipes with
deterministic humanization:

- a harmonic profile specifies which sine waves are mixed together
- an ADSR envelope shapes loudness over the note's lifetime and release tail
- a deterministic seeded `VariationProfile` introduces micro-variations (pitch, amplitude, attack timing, overtones)
- a pure-Python SplitMix64 PRNG drives variations with zero ambient entropy
- strict CWE-400 / CWE-674 / CWE-1333 resource and recursion bounds are enforced
- formal VER verification artifacts (`musical_instruments.warrant`, `required_capabilities.json`)

## Keyboard-Style Catalog

The package exposes a full 128-entry General-MIDI-style melodic program catalog.
That gives us the same kind of "program number selects a voice" idea found on
many keyboards.

Examples:

```python
from musical_instruments import (
    VariationProfile,
    get_gm_program,
    instrument_for_gm_program,
    render_instrument_note,
)

program = get_gm_program(74)              # Flute
instrument = instrument_for_gm_program(74)

# Render with deterministic seeded micro-variation
variation = VariationProfile(
    pitch_cents_jitter=5.0,
    amplitude_jitter=0.08,
    attack_jitter_ms=2.0,
    seed=42,
)

rendered = render_instrument_note(
    "A4",
    duration_seconds=0.5,
    instrument=instrument,
    amplitude=0.2,
    variation_profile=variation,
)

print(program.name)                       # Flute
print(rendered.instrument.id)             # flute_naive
print(rendered.pcm_buffer.samples[:5])
```

## Built-In Naive Presets

The built-in naive presets cover common instrument archetypes:

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
| `silence` | zero-amplitude silence utility |

## Security & Verification Invariants (VER15)

In accordance with the Verified Shipping Program:

- **CWE-400 (Uncontrolled Resource Consumption):** Strictly bounded by
  `MAX_PARTIALS = 64`, `MAX_NOTE_DURATION_SECONDS = 60.0`,
  `MAX_SAMPLE_RATE_HZ = 192_000.0`, and `MAX_SAMPLE_COUNT = 16_777_216`.
- **CWE-190 (Integer Overflow):** Bounded envelope calculations and protected
  sample rounding (`_safe_round_sample`) eliminate float-to-int overflow.
- **CWE-674 (Uncontrolled Recursion):** Synthesis and envelope evaluation are
  strictly iterative with constant call stack depth.
- **CWE-1333 (ReDoS) / CWE-94 (Code Injection):** Zero regular expressions
  (`re`) and zero dynamic code execution (`eval`/`exec`).
- **CWE-208 (Timing Side-Channels):** All operands are public (`musical_instruments.warrant`),
  establishing the `warrant-spec::all-operands-public` derived attestation.
- **Required Capabilities:** `capabilities: []` declared in `required_capabilities.json`
  (pure computation).

## Development

```bash
bash BUILD
```

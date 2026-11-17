# musical-instruments

Naive musical instrument timbre models and additive note rendering in safe Rust (`MUS05`).

## Overview

`MUS05` defines the first musical instrument timbre model:

```text
note frequency + harmonic profile + ADSR envelope -> shaped waveform -> PCM audio
```

The goal is not realism yet, but to provide a deterministic, inspectable model for virtual keyboards, score synthesis, and virtual orchestras:
- Notes retain identical fundamental frequencies across instruments (equal temperament / A440).
- Instruments differ by harmonic overtones, ADSR amplitude envelopes, output gains, and subtle micro-variations.
- Deterministic humanization via pure-Rust `SplitMix64` PRNG.
- Seamless conversion to continuous `oscillator::SampleBuffer` and discrete `pcm_audio::PcmAudio`.

## Timbre Model

- **Harmonic Profile** (`HarmonicPartial`): Sum of sinusoidal overtones with integer or non-integer frequency multipliers, amplitudes, and phase offsets.
- **Envelope Profile** (`AdsrEnvelope`): 4-stage loudness envelope (attack, decay, sustain, release) with release tail extension.
- **Variation Profile** (`VariationProfile`): Pitch cents jitter, amplitude jitter, attack timing jitter, and overtone jitter driven deterministically by `SplitMix64`.
- **Instrument Profiles** (`InstrumentProfile`): 21 built-in naive presets spanning pipes, reeds, strings, pianos, organs, chromatic and pitched percussion, synth leads/pads, and sound effects.
- **General MIDI Map** (`GMProgram`): Complete 128-program General MIDI melodic catalog mapped to built-in presets.

## Security & Verification Guarantees

- `#![forbid(unsafe_code)]`
- Zero third-party crate dependencies (pure `std` + workspace-local `oscillator` / `pcm-audio`).
- Strict CWE defenses:
  - **CWE-400 (Uncontrolled Resource Consumption)**:
    - `MAX_PARTIALS = 64`
    - `MAX_NOTE_DURATION_SECONDS = 60.0`
    - `MAX_SAMPLE_RATE_HZ = 192_000.0`
    - `MAX_SAMPLE_COUNT = 16_777_216` (2^24)
    - Pre-allocation checks on sample counts, envelope tails, and partial counts.
  - **CWE-190 (Integer Overflow)**: Checked arithmetic and rounding across sample indexes and buffer durations.
  - **CWE-20 (Improper Input Validation)**: All numeric inputs are checked for finiteness. `NaN`, `±Inf`, and negative values fail closed.
  - **CWE-674 (Uncontrolled Recursion)**: Strictly iterative synthesis loops; zero recursive calls.
- Warrant specifications: `musical-instruments.warrant` and `code/specs/musical-instruments.warrant`.

## Usage

```rust
use musical_instruments::{render_instrument_note, RenderOptions, VariationProfile};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Render flute A4 (440 Hz) for 0.01 seconds at 8 kHz
    let opts = RenderOptions::new()
        .instrument_id("flute_naive")
        .sample_rate_hz(8000.0)
        .amplitude(0.25);

    let render = render_instrument_note("A4", 0.01, &opts)?;
    assert_eq!(render.fundamental_hz, 440.0);
    assert_eq!(render.floating_samples.sample_count(), 720); // 0.01s + 0.08s release
    assert_eq!(render.pcm_buffer.sample_count(), 720);

    // Deterministic humanized variation
    let var = VariationProfile::new(15.0, 0.2, 5.0, Some(424242), 0.1)?;
    let var_opts = RenderOptions::new()
        .instrument_id("violin_naive")
        .sample_rate_hz(16000.0)
        .variation_profile(var);

    let violin_render = render_instrument_note("C4", 0.05, &var_opts)?;
    assert_eq!(violin_render.pcm_buffer.sample_count(), 2720);

    Ok(())
}
```

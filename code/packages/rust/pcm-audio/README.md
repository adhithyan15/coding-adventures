# pcm-audio

Pulse Code Modulation (PCM) audio container, quantization, and secure WAV serialization/parsing in safe Rust (`MUS01` Stage 5).

## Overview

PCM is the fundamental digital audio stage in the note-to-sound signal chain:

```text
typed note -> frequency -> oscillator -> sampler -> PCM audio -> DAC -> speaker
```

It takes smooth normalized floating-point samples (in range $[-1.0, 1.0]$) and quantizes each measurement into finite signed 16-bit integers (`[-32768, 32767]`).

This crate also provides secure, standard-compliant RIFF/WAVE serializer and deserializer functions for uncompressed 16-bit and 32-bit float audio.

## Features

- **PCM Container** (`PcmAudio`, `PCMBuffer`): Represents digital audio with sample rate, channel count, bit depth, and signed integer/float samples.
- **Quantization & Clamping** (`safe_round_sample`, `float_to_pcm16`, `pcm16_to_f32`): Quantizes floating-point samples to signed 16-bit integers with saturation and clipping tracking.
- **WAV Codec** (`to_wav_bytes`, `from_wav_bytes`): Complete RIFF/WAVE serializer and parser.
- **Interoperability**: Direct integration with `oscillator::SampleBuffer`.

## Security & Verification Guarantees

- `#![forbid(unsafe_code)]`
- Zero third-party crate dependencies (pure `std`).
- Defensive verification against standard weaknesses:
  - **CWE-400 (Resource Exhaustion / Allocation Bombs)**:
    - `MAX_SAMPLE_RATE_HZ = 192_000.0`
    - `MAX_DURATION_SECONDS = 60.0`
    - `MAX_SAMPLES = 16_777_216`
    - `MAX_WAV_BYTES = 64 * 1024 * 1024` (64 MiB)
    - WAV parser never pre-allocates based solely on unchecked declared chunk sizes.
  - **CWE-190 (Integer Overflow)**:
    - Arithmetic on chunk lengths, sample rates, byte rates, and block alignments uses checked operations.
  - **CWE-20 (Improper Input Validation)**:
    - Rejects malformed WAV headers, truncated slices, unsupported format tags, invalid channel counts, and non-finite floats.
- Fully deterministic and side-effect free.

## Usage

```rust
use pcm_audio::{PcmAudio, PcmFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let format = PcmFormat::default(); // 44.1 kHz, mono, 16-bit
    let samples = vec![0.0, 0.5, -0.5, 1.0];
    let audio = PcmAudio::from_f64_slice(&samples, format, 0.0)?;
    
    // Serialize to standard WAV bytes
    let wav_bytes = audio.to_wav_bytes()?;
    
    // Parse WAV bytes back
    let decoded = PcmAudio::from_wav_bytes(&wav_bytes)?;
    assert_eq!(decoded.sample_count(), 4);
    Ok(())
}
```

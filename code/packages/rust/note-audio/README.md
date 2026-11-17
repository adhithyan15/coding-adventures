# note-audio

Safe Rust orchestration of the note-to-sound signal chain (MUS01 Stages 1-5).

Converts musical note names, MIDI note numbers, or raw frequencies along with
duration, waveform family, and amplitude into continuous oscillator models and
discrete PCM audio buffers (`pcm_audio::PcmAudio`).

## Pipeline

```text
typed note -> frequency -> oscillator -> sampler -> PCM audio
```

## Security & Verification Guarantees

- **Safety**: `#![forbid(unsafe_code)]`, zero external dependencies (pure `std`).
- **Strictly pure capabilities**: `[]` declared in `required_capabilities.json`.
- **CWE-400 (Resource Exhaustion)**: Rates, durations, and sample budgets are validated up front.
- **CWE-190 (Integer Overflow)**: Checked arithmetic on sample counts.
- **CWE-20 (Improper Input Validation)**: Rejects `NaN`, `+-Inf`, negative durations, and Nyquist violations.
- **Verification Warrant**: `note-audio.warrant` and `code/specs/note-audio.warrant`.

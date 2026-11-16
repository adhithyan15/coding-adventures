# virtual-speaker

Virtual speaker models and sound-pressure proxies (MUS01 Stage 7).

Turns virtual continuous analog voltage signals into pressure proxies:

```text
... -> PCM audio -> DAC -> speaker
```

## Models

- `LinearSpeakerSignal`: scales input signal by a configurable finite `speaker_gain` (default `1.0`).
- `ClippingSpeakerSignal`: scales by gain and clips to `[-limit, +limit]`.
- `render_speaker_output`: samples speaker signal on a uniform grid bounded by `MAX_SAMPLE_RATE_HZ` and `MAX_SAMPLES`.

## Security & Verification Guarantees

- **Safety**: `#![forbid(unsafe_code)]`, zero external dependencies (pure `std`).
- **Strictly pure capabilities**: `[]` declared in `required_capabilities.json`.
- **CWE-400 (Resource Exhaustion)**: Sampling bounded by `MAX_SAMPLES` and `MAX_SAMPLE_RATE_HZ`.
- **CWE-20 (Improper Input Validation)**: Rejects `NaN`, `+-Inf` gains, limits, and query times fail-closed.
- **Verification Warrant**: `virtual-speaker.warrant` and `code/specs/virtual-speaker.warrant`.

# virtual-dac

Virtual Digital-to-Analog Converter (DAC) models (MUS01 Stage 6).

Turns discrete signed 16-bit PCM samples from [`pcm_audio::PcmAudio`] back into
virtual continuous analog voltage signals implementing [`oscillator::ContinuousSignal`].

## Reconstruction Models

- `ZeroOrderHoldDACSignal`: holds each PCM sample as a constant voltage until the next sample time.
- `LinearInterpolationDACSignal`: linearly interpolates voltages between adjacent samples.

## Security & Verification Guarantees

- **Safety**: `#![forbid(unsafe_code)]`, zero external dependencies (pure `std`).
- **Strictly pure capabilities**: `[]` declared in `required_capabilities.json`.
- **CWE-190 / Rounding Defense**: Float sample indices clamped safely with `min(max(idx, 0), count - 1)` (`clamp_sample_index`).
- **CWE-20 (Improper Input Validation)**: Rejects `NaN`, `+-Inf` query times fail-closed.
- **Verification Warrant**: `virtual-dac.warrant` and `code/specs/virtual-dac.warrant`.

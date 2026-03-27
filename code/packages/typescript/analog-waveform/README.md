# @coding-adventures/analog-waveform

Continuous-time waveform primitives for the electronics track.

This package makes the analog-signal concept explicit without breaking the
existing introductory `wave` package.

## Included waveforms

- `ConstantWaveform`
- `SineWaveform`

## Core idea

An analog waveform is anything that can answer:

```ts
sampleAt(timeSeconds: number): number
```

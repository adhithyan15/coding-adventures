# @coding-adventures/oscillator

Virtual continuous oscillators, stateful phase accumulator, deterministic PRNG noise, and uniform samplers for audio synthesis and clock simulation (OSC00).

## Features

- **Continuous Waveforms**: `SineOscillator`, `SquareOscillator`, `TriangleOscillator`, `SawtoothOscillator`, and `PulseOscillator`.
- **Stateful Phase Accumulator**: `OscillatorState` with `step()` and `renderSamples()`.
- **Deterministic Noise**: `SplitMix64` and `LcgPrng` for repeatable pseudo-random noise with zero ambient dependencies.
- **Uniform Sampler**: `UniformSampler` and `SampleBuffer` with half-open intervals and exact timing calculations.
- **Security & Hardening**: Strict CWE-20 input validation, fail-closed numerical bounds, pre-flight allocation verification against CWE-400 resource exhaustion (`MAX_SAMPLES = 16_777_216`, `MAX_SAMPLE_RATE_HZ = 192_000`, `MAX_FREQ_HZ = 96_000`, `MAX_DURATION_SECONDS = 60.0`). Zero external dependencies, pure capability profile (`[]`).

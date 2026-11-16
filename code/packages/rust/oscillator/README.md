# oscillator

Virtual continuous oscillators and uniform samplers in safe Rust (`OSC00`).

## Overview

An oscillator in this crate is not a sound card, timer, or circuit. It is an ideal mathematical signal: given a time in seconds, it answers what value the signal would have at that exact instant:

$$x(t) \rightarrow \text{value}$$

A sampler sits one layer above the oscillator. It chooses concrete, evenly spaced moments in time and samples the signal onto a uniform time grid:

$$t_n = \text{start\_time\_seconds} + \frac{n}{\text{sample\_rate\_hz}}$$

This cleanly separates the continuous physical intuition of waves from discrete digital buffers.

## Waveforms

- **Sine** (`SineOscillator`): Smooth harmonic motion, $x(t) = \text{offset} + A \sin(2\pi (f t + \phi))$
- **Square** (`SquareOscillator`): Bipolar or arbitrary high/low switching with configurable duty cycle
- **Pulse** (`PulseOscillator`): Rectangular pulse wave with duty cycle and amplitude
- **Sawtooth** (`SawtoothOscillator`): Linear rising ramp wave from $-1$ to $+1$
- **Triangle** (`TriangleOscillator`): Linear symmetric triangular wave in phase with sine

## Security and Verification Guarantees

- `#![forbid(unsafe_code)]`
- Zero third-party crate dependencies (pure `std`).
- Explicit CWE bounds:
  - **CWE-400**: Resource exhaustion defense with `MAX_SAMPLE_RATE_HZ = 192_000.0`, `MAX_DURATION_SECONDS = 60.0`, and `MAX_SAMPLES = 16_777_216`. Allocation capacities are validated before any `Vec` allocation.
  - **CWE-190**: Integer overflow protection in sample count math.
  - **CWE-20**: Strict fail-closed input validation. Non-finite floats (`NaN`, `±Inf`) are rejected cleanly with `OscillatorError`.
- Deterministic and pure: no wall-clock reads, no sleep, no global mutable state.

## Usage

```rust
use oscillator::{ContinuousSignal, SineOscillator, UniformSampler};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 440 Hz concert A sine wave
    let osc = SineOscillator::new(440.0)?;
    
    // Sample at 44.1 kHz for 0.01 seconds
    let sampler = UniformSampler::new(44_100.0)?;
    let buffer = sampler.sample(&osc, 0.01)?;
    
    assert_eq!(buffer.sample_count(), 441);
    assert_eq!(buffer.sample_rate_hz(), 44_100.0);
    Ok(())
}
```

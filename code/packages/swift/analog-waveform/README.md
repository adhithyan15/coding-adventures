# Analog Waveform

Models of continuous-time signals for simulation and processing.

## Overview

A standalone package to represent explicit analog properties based on functions of continuous time: `x(t)`.

Includes basic building-blocks:
- `ConstantWaveform`: DC signals.
- `SineWaveform`: General AC signals varying per phase, amplitude, and frequency.

## Usage

```swift
import AnalogWaveform

let waveform = SineWaveform(amplitude: 5.0, frequency: 50.0) // 50 Hz, 5V peak
let v = waveform.sampleAt(0.005) // Returns voltage sequentially based on precise time
```

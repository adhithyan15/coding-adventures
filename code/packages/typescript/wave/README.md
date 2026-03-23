# wave

Simple harmonic wave model for electromagnetic wave fundamentals.

## Overview

This package models a sinusoidal (simple harmonic) wave using the equation:

```
y(t) = A · sin(2πft + φ)
```

where A is amplitude, f is frequency, and φ is phase offset.

It builds on the `trig` package, which computes sine from first principles using Taylor series — no `Math.sin` anywhere in the dependency chain.

## Installation

```bash
npm install
```

## Usage

```typescript
import { Wave } from "wave/src/wave";

// Create a 440 Hz wave (concert A) with amplitude 1.0
const wave = new Wave(1.0, 440.0);

// Evaluate at various times
wave.evaluate(0.0);    // 0.0 — starts at zero
wave.evaluate(0.25 / 440);  // 1.0 — quarter period = peak

// Derived quantities
wave.period();           // 1/440 ≈ 0.00227 seconds
wave.angularFrequency(); // 2π × 440 ≈ 2764.6 rad/s
```

## API

### `new Wave(amplitude, frequency, phase?)`

- **amplitude** (number): Peak displacement. Must be >= 0.
- **frequency** (number): Cycles per second (Hz). Must be > 0.
- **phase** (number, optional): Initial phase offset in radians. Defaults to 0.

### `wave.evaluate(t: number): number`

Computes the wave's value at time `t` seconds.

### `wave.period(): number`

Returns the period `T = 1/f` in seconds.

### `wave.angularFrequency(): number`

Returns `ω = 2πf` in radians per second.

## Dependencies

- **trig** — Provides `sin()` and `PI`, computed from first principles via Taylor series.

## How It Fits

This is a Layer 2 package in the coding-adventures stack. It depends on the Layer 1 `trig` package and provides the foundation for future electromagnetic wave, signal processing, and Fourier analysis packages.

## Testing

```bash
npm test
```

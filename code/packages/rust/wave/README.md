# wave

Sinusoidal wave modeling from first principles, built on the `trig` package.

## What is this?

This package provides a `Wave` struct that models a sinusoidal wave — the fundamental building block of sound, light, radio signals, and countless other physical phenomena. All trigonometric computations use the `trig` package (Maclaurin series from scratch, no `std::f64::sin`).

## How it fits in the stack

```
wave  (this package)
  └── trig  (sin, cos, PI — all from first principles)
```

The `wave` package is a consumer of `trig`, demonstrating how low-level math primitives compose into higher-level physics models.

## Usage

```rust
use wave::Wave;
use trig::PI;

// Create a 440 Hz sine wave (concert A) with unit amplitude
let a440 = Wave::new(1.0, 440.0, 0.0).unwrap();

// Evaluate at t = 0 seconds
assert!(a440.evaluate(0.0).abs() < 1e-10);  // sin(0) = 0

// Get the period (duration of one cycle)
let period = a440.period();  // ~0.00227 seconds

// Get angular frequency
let omega = a440.angular_frequency();  // 2 * PI * 440

// Phase-shifted wave (starts at peak, like cosine)
let cosine_wave = Wave::new(1.0, 1.0, PI / 2.0).unwrap();
assert!((cosine_wave.evaluate(0.0) - 1.0).abs() < 1e-10);
```

## API

- `Wave::new(amplitude, frequency, phase) -> Result<Wave, &str>` — constructor with validation
- `wave.period() -> f64` — duration of one cycle (1/frequency)
- `wave.angular_frequency() -> f64` — frequency in radians/second (2 * PI * frequency)
- `wave.evaluate(t) -> f64` — compute the wave's value at time t

## The physics

A sinusoidal wave is described by: `y(t) = A * sin(2 * pi * f * t + phi)`

| Parameter | Symbol | Meaning |
|-----------|--------|---------|
| Amplitude | A | Peak displacement (>= 0) |
| Frequency | f | Cycles per second in Hz (> 0) |
| Phase | phi | Time offset in radians |
| Period | T = 1/f | Duration of one cycle |
| Angular frequency | omega = 2*pi*f | Radians per second |

# Wave

Sinusoidal wave modeling from first principles. This package provides a `Wave` class that represents a single sinusoidal wave, parameterized by amplitude, frequency, and phase.

## How It Fits

This package builds on the [trig](../trig/) package, using its `Trig.sin` function (implemented via Maclaurin series) to evaluate wave values. No standard library math functions are used.

## Dependencies

- `trig` — provides `Trig.sin` and `Trig::PI`

## Usage

```ruby
require_relative 'lib/wave'

# Create a 440 Hz wave (concert A) with amplitude 1.0
wave = Wave.new(1.0, 440.0)

# Evaluate at time t = 0.001 seconds
y = wave.evaluate(0.001)

# Query derived properties
wave.period             # => ~0.00227 seconds
wave.angular_frequency  # => ~2764.6 rad/s

# Phase-shifted wave (starts at peak, like cosine)
cosine_wave = Wave.new(1.0, 440.0, Trig::PI / 2.0)
cosine_wave.evaluate(0.0)  # => 1.0
```

## API

### `Wave.new(amplitude, frequency, phase = 0.0)`

Creates a new sinusoidal wave.

- `amplitude` — peak height (must be >= 0)
- `frequency` — cycles per second in Hz (must be > 0)
- `phase` — phase offset in radians (default: 0.0)

Raises `ArgumentError` if amplitude is negative or frequency is not positive.

### `#evaluate(t)`

Returns the wave's value at time `t` seconds: `A * sin(2 * PI * f * t + phase)`.

### `#period`

Returns `1.0 / frequency` (seconds per cycle).

### `#angular_frequency`

Returns `2 * PI * frequency` (radians per second).

### `#amplitude`, `#frequency`, `#phase`

Read-only accessors for the wave's defining parameters.

## Running Tests

```bash
ruby test/test_wave.rb
```

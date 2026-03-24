# Wave

Sinusoidal wave generator built on the from-scratch `Trig` library.

## What It Does

Models a sinusoidal wave defined by three parameters --- amplitude, frequency,
and phase --- and evaluates the wave equation `y(t) = A * sin(2*pi*f*t + phi)`
at any point in time.

## How It Fits

This is a **PHY01** (physics layer 1) package in the coding-adventures stack:

- Depends on: `trig` (PHY00) for `sin()` and `pi()`
- Depended on by: future signal processing, audio synthesis, and physics
  simulation packages

## Usage

```elixir
# Create a 440 Hz wave with amplitude 1.0
{:ok, wave} = Wave.new(1.0, 440.0)

# Evaluate at time t = 0.001 seconds
y = Wave.evaluate(wave, 0.001)

# Get derived quantities
Wave.period(wave)            # => 0.002272... seconds
Wave.angular_frequency(wave) # => 2764.6... rad/s

# Phase-shifted wave (starts at peak, like cosine)
{:ok, cosine_wave} = Wave.new(1.0, 1.0, Trig.pi() / 2.0)
Wave.evaluate(cosine_wave, 0.0)  # => 1.0
```

## API

| Function               | Description                                    |
|------------------------|------------------------------------------------|
| `Wave.new/2,3`         | Create a wave; returns `{:ok, wave}` or error  |
| `Wave.evaluate/2`      | Compute displacement at time t                 |
| `Wave.period/1`        | Period in seconds (1/frequency)                |
| `Wave.angular_frequency/1` | Angular frequency in rad/s (2*pi*f)       |

## Running Tests

```bash
mix deps.get
mix test
```

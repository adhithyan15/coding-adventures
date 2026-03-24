# Trig

Trigonometric functions implemented from first principles using Maclaurin (Taylor) series. No external dependencies, no `:math` module — just basic arithmetic building up to `sin`, `cos`, and angle conversions.

## Layer

PHY00 — Physics layer 0. This is a leaf package with no dependencies.

## How It Works

The **Maclaurin series** (Taylor series centered at 0) lets us compute transcendental functions using only addition, multiplication, and division:

```
sin(x) = x - x^3/3! + x^5/5! - x^7/7! + ...
cos(x) = 1 - x^2/2! + x^4/4! - x^6/6! + ...
```

We use 20 terms with **range reduction** (normalizing input to [-pi, pi]) for numerical stability across all input magnitudes.

## API

| Function       | Description                              |
|----------------|------------------------------------------|
| `Trig.pi()`    | Pi to double-precision (IEEE 754 float64)|
| `Trig.sin(x)`  | Sine via Maclaurin series, radians input |
| `Trig.cos(x)`  | Cosine via Maclaurin series, radians input|
| `Trig.radians(deg)` | Convert degrees to radians          |
| `Trig.degrees(rad)` | Convert radians to degrees          |

## Usage

```elixir
Trig.sin(Trig.pi() / 2)   #=> 1.0
Trig.cos(0)                #=> 1.0
Trig.radians(180)          #=> 3.141592653589793
Trig.degrees(Trig.pi())    #=> 180.0
```

## Running Tests

```sh
mix test
```

# Trig

## Exact small arctangent

`Trig.atan/1` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

`Trig.sqrt/1` normalizes finite binary64 values by powers of four into
`[0.25, 4)` before at most 60 Newton steps, preserves negative zero, and rejects
negative inputs with `ArithmeticError`. Erlang/OTP does not expose IEEE-754 NaN
or infinity as ordinary BEAM floats, so those two shared fixture cases are
explicitly not applicable in this lane. Finite boundary behavior is checked
against [`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions implemented from first principles, bit for bit the
reference algorithm **TRIG-REF-1** of
[PHY00a](../../../specs/PHY00a-trig-convergence.md). No `:math` module, no
external dependencies: only `+ - * /` on IEEE 754 doubles.

## Layer

PHY00 — Physics layer 0. This is a leaf package with no dependencies.

## How It Works

A sine repeats, so `sin` is computed from a reduced angle and a quadrant:

1. **Reduce.** `k = nearest integer to x / (pi/2)` (the add-and-subtract
   `(t + MAGIC) - MAGIC` trick), then `r = x - k*pi/2` by Cody-Waite
   subtraction with pi/2 stored as five pieces, so `|r| <= pi/4`.
2. **Pick the quadrant** `q = k &&& 3`.
3. **Evaluate** a ten-term Maclaurin polynomial in `r` by Horner's rule on stored
   coefficients (`1/3!`, `1/5!`, ...). No division.

```
sin(x) = [ sinK(r), cosK(r), -sinK(r), -cosK(r) ][q]
cos(x) = [ cosK(r), -sinK(r), -cosK(r), sinK(r) ][q]
```

`radians`/`degrees` are one multiply by a stored constant. `tan` is `sin / cos`
with a `+-1e308` sentinel where `|cos| < 1e-15`.

## Domain and the BEAM

`sin`, `cos` and `tan` are defined for `|x| <= 2^29` (about 5.4e8 radians). The
reference returns NaN outside it. **A BEAM float cannot be NaN or infinite**, so
this port **raises `ArithmeticError`** for `|x| > 2^29`, as it does for every
non-finite result (`degrees(1.7e308)`; `atan2` when `y / x` overflows).

## Accuracy

Within 3 ulp of the correctly rounded value for `sin` and `cos` on the whole
domain, within 1 ulp for `radians` and `degrees`. `tan` makes no accuracy claim.
`sqrt`, `atan` and `atan2` (Part II) are unchanged from the earlier port and are
pinned bit for bit, not claimed accurate.

## API

| Function       | Description                              |
|----------------|------------------------------------------|
| `Trig.pi()`    | Pi to double-precision (IEEE 754 float64)|
| `Trig.sin(x)`  | Sine, radians; raises beyond `2^29`      |
| `Trig.cos(x)`  | Cosine, radians; raises beyond `2^29`    |
| `Trig.tan(x)`  | Tangent with pole sentinel               |
| `Trig.radians(deg)` | Degrees to radians                  |
| `Trig.degrees(rad)` | Radians to degrees                  |
| `Trig.sqrt/1`, `Trig.atan/1`, `Trig.atan2/2` | Part II        |
| `Trig.constants()` | Every PHY00a constant by spec name   |

## Usage

```elixir
Trig.sin(Trig.pi() / 2)   #=> 1.0
Trig.cos(0)                #=> 1.0
Trig.radians(180)          #=> 3.141592653589793
Trig.degrees(Trig.pi())    #=> 180.0
Trig.sin(1.0e20)           # raises ArithmeticError (outside the domain)
```

## Running Tests

```sh
mix test
```

`test/conformance_test.exs` checks all 62,154 shared vectors in
`code/specs/fixtures/trig-reference-v1` bit for bit. Rows whose input is NaN or
infinity cannot be built as a BEAM float; they are counted and skipped by name
(PHY00a 6.5 (d)), and rows whose result is non-finite require the raise.

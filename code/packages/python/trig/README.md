# trig

Trigonometric functions implemented from first principles: no math library, and
`sin`, `cos`, `radians` and `degrees` do not even divide. One algorithm
(**TRIG-REF-1**, [PHY00a](../../../specs/PHY00a-trig-convergence.md)) is shared by
all seventeen language ports, so every port returns the same bits, which is checked
against the shared vectors in
[`trig-reference-v1`](../../../specs/fixtures/trig-reference-v1/README.md).

## Layer

**PHY00** — this is a leaf package with no dependencies. It is used by `wave`,
`oscillator`, the audio and music packages, the 2D geometry packages and the CAS
numeric packages.

## API

| Function       | Description                                                          |
|----------------|----------------------------------------------------------------------|
| `PI`, `HALF_PI`| Constants, pinned by bit pattern                                      |
| `sin(x)`       | Sine of `x` radians, `\|x\| <= 2**29`, else NaN                      |
| `cos(x)`       | Cosine of `x` radians, `\|x\| <= 2**29`, else NaN                    |
| `tan(x)`       | `sin/cos`, with a documented `+-1e308` sentinel at the poles          |
| `radians(deg)` | One multiply by `pi/180`                                              |
| `degrees(rad)` | One multiply by `180/pi`                                              |
| `sqrt(x)`      | Newton's method (`ValueError` for negatives)                          |
| `atan(x)`      | Arctangent (half-angle reduction + series)                            |
| `atan2(y, x)`  | Four-quadrant arctangent; NaN if either argument is NaN               |

## Usage

```python
from trig import sin, cos, radians, degrees, PI

print(sin(PI / 2))      # 1.0
print(cos(0))            # 1.0
print(sin(radians(30)))  # 0.49999999999999994 (radians(30) is not exactly pi/6)
print(degrees(PI))       # 180.0
```

## How it works

A sine repeats, so an angle is split into whole quarter-turns and a small leftover:

    x = k * (pi/2) + r,   |r| <= pi/4,   k = nearest integer to x / (pi/2)

* **Finding `k`** with `(x * (2/pi) + MAGIC) - MAGIC`, where `MAGIC = 1.5 * 2**52`:
  adding then subtracting it rounds to the nearest integer using only `+` and `-`.
* **Finding `r`** by subtracting `k * piece` for five pieces of `pi/2`. The first
  four have at most 24 significant bits, so `k * piece` is exact for `|k| < 2**29`
  (the Cody-Waite device); the fifth is the remainder.
* **The kernels** are the Maclaurin series on `|r| <= pi/4`, summed by Horner's
  rule from stored coefficients, `1/3!, 1/5!, ...`, ten terms each:

      sin r = r + r*z*(S1 + S2*z + ... + S9*z**8)         z = r*r
      cos r = 1 - z/2 + z*z*(C2 + C3*z + ... + C9*z**7)

* **The quadrant** `k mod 4` (`int(k) & 3`) selects `sin r`, `cos r`, `-sin r`,
  `-cos r` for sine, and `cos r`, `-sin r`, `-cos r`, `sin r` for cosine.

Every constant is a bit pattern, every operation is a single IEEE rounding, and every
product that feeds a sum sits in its own statement, so no port can differ.

## Accuracy and domain

* Within **3 ulp** of the exact value for `|x| <= 2**29` (about 5.4e8 radians); the
  worst case found is 2.38 ulp. It is not a correctly rounded libm.
* `sin`, `cos` and `tan` return **NaN** for NaN, the infinities and `|x| > 2**29`: a
  finite answer there would be wrong but look right. Reduce long-running phases
  yourself (see `oscillator`, which keeps its phase in cycles).
* `sin(-0.0)` is `-0.0`; `|x| < 2**-27` returns `x`; `radians`/`degrees` keep the
  sign of zero.
* `tan`'s `+-1e308` at the poles, and `atan2(-0, x < 0) = +pi`,
  `atan2(0, 0) = 0`, `atan2(+-inf, +-inf) = NaN` are pinned *conventions*, not IEEE
  754 results and not accuracy claims.

## Tests

`pytest tests/` runs the example tests plus `tests/test_conformance.py`, which decodes
all 62,154 shared vectors by bit cast and requires bit-exact results.

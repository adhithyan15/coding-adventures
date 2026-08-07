# trig

## Exact small arctangent

`atan` returns `x` unchanged when `|x| <= 2^-27`, before any half-angle
reduction. This exact binary64 identity preserves negative zero and both signs
of the minimum subnormal; the shared PHY00 fixture locks those boundaries.

## Full-range square root

The square-root API follows PHY00 across the full binary64 range. It preserves
negative zero, returns positive infinity and NaN unchanged, rejects negative
inputs with the lane-native error, normalizes by powers of four into `[0.25, 4)`,
and then applies at most 60 Newton steps without calling the host square-root
routine. Boundary behavior is checked against
[`trig.json`](../../../specs/fixtures/phy00-phy01-v1/cases/trig.json).

Trigonometric functions implemented from first principles with basic
arithmetic.

## API

- `piValue`, `twoPi`, and `halfPi` provide the shared angle constants.
- `sin` and `cos` use 20-term Maclaurin series after reducing angles to
  `[-pi, pi]`.
- `radians` and `degrees` convert angle units.
- `sqrt` uses Newton's method and rejects negative inputs explicitly.
- `tan` uses the local sine and cosine implementations and returns a large
  signed finite value near a pole.
- `atan` uses complementary and half-angle reduction before its series;
  `atan2` selects the correct quadrant.

## Dependencies

The library depends only on `base`. Tests use Hspec.

## Running the tests

```sh
cabal test all
```

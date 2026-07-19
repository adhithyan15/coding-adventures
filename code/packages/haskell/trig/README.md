# trig

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

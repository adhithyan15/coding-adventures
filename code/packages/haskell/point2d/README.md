# point2d (Haskell)

Pure Haskell implementation of the G2D00 two-dimensional geometry primitives.
The package provides immutable points/vectors and axis-aligned rectangles
without calling a native graphics library.

## API

- `Point` stores an `(x, y)` coordinate pair.
- `origin`, `add`, `subtract`, `scale`, and `negate` provide vector arithmetic.
- `dot`, `cross`, `magnitude`, `normalize`, `distance`, `lerp`,
  `perpendicular`, and `angle` provide vector geometry.
- `Rect` stores an origin and extent.
- `rectFromPoints`, `zeroRect`, `minPoint`, `maxPoint`, and `center` construct
  and inspect rectangles.
- `isEmpty`, `containsPoint`, `union`, `intersection`, and `expandBy` provide
  half-open AABB predicates and set operations.

`magnitude` and `angle` compose the existing pure Haskell `trig` package.
Rectangle containment uses `[x, x + width) x [y, y + height)`, so the top and
left edges are included while the right and bottom edges are excluded.

## Development

```sh
cabal test all
cabal test all --enable-coverage
```

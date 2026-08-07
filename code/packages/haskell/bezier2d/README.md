# bezier2d (Haskell)

Pure Haskell implementation of G2D02 quadratic and cubic Bezier curves. The
package composes the existing `point2d` package and uses de Casteljau's
algorithm for numerically stable curve evaluation and subdivision.

## API

- `QuadraticBezier` and `CubicBezier` are immutable control-point records.
- `evaluateQuadratic` and `evaluateCubic` evaluate points with de Casteljau's
  algorithm.
- `derivativeQuadratic` and `derivativeCubic` return unnormalized tangent
  vectors.
- `splitQuadratic` and `splitCubic` produce exact left and right sub-curves.
- `toPolylineQuadratic` and `toPolylineCubic` adaptively flatten curves using
  the G2D02 midpoint error criterion.
- `boundingBoxQuadratic` and `boundingBoxCubic` compute tight bounds from
  interior derivative roots as well as endpoints.
- `elevate` converts a quadratic exactly into an equivalent cubic.

All computations are pure. The only package dependency is `point2d`; square
roots needed by cubic bounding boxes use Haskell's standard floating-point
arithmetic.

## Development

```sh
cabal test all
cabal test all --enable-coverage
```

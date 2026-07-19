# affine2d (Haskell)

Pure Haskell implementation of the G2D01 two-dimensional affine
transformation matrix. The package uses the standard SVG/Canvas six-scalar
representation `[a, b, c, d, e, f]`:

```text
x' = a*x + c*y + e
y' = b*x + d*y + f
```

## API

- `Affine` is an immutable value containing the six matrix components.
- `identity`, `translate`, `rotate`, `rotateAround`, `scale`, `scaleUniform`,
  `skewX`, and `skewY` construct common transforms.
- `multiply` applies its right transform first; `thenTransform` gives readable
  left-to-right sequencing (`then` is a Haskell keyword).
- `applyToPoint` includes translation while `applyToVector` applies only the
  linear portion.
- `determinant` and `invert` expose area scaling and checked inversion.
- `isIdentity`, `isTranslationOnly`, and `toArray` support renderer fast paths
  and graphics API interop.

Rotation and skew compose the existing pure Haskell `trig` package, while
points come from the pure Haskell `point2d` package. Inversion rejects matrices
whose determinant magnitude is below `1e-12`; predicates use the G2D01
`1e-10` tolerance.

## Development

```sh
cabal test all
cabal test all --enable-coverage
```

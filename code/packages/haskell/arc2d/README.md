# arc2d (Haskell)

Pure Haskell implementation of G2D03 elliptical arcs. It supports both SVG
endpoint form and center form, including the W3C endpoint-to-center conversion.

## API

- `SvgArc` stores SVG `A` command parameters; `toCenterArc` performs the W3C
  conversion with radius correction and degeneracy guards.
- `CenterArc` supports parametric evaluation, unnormalized tangents, exact
  rotated-ellipse arc bounds, and cubic Bezier approximation.
- `evaluateSvgArc` and `boundingBoxSvgArc` treat degenerate arcs as line
  segments, while `toCubicBeziersSvg` returns an empty list for them.

All angular and square-root operations use the repository's pure Haskell
`trig` package. Points, rectangles, and cubic curves come from `point2d` and
`bezier2d`.

## Development

```sh
cabal test all
cabal test all --enable-coverage
```

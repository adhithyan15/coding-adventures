# bezier2d (Swift)

Pure G2D02 quadratic and cubic Bézier curve evaluation, differentiation,
splitting, conservative polyline flattening, tight bounds, and quadratic degree
elevation over the shared `Point2D` primitives.

`polyline(tolerance:)` is a throwing API. It measures every control point
against the finite endpoint segment, subdivides iteratively in parameter order,
and never returns a partial prefix on failure. Tolerance and controls must be
finite, tolerance must be positive, and derived geometry must remain finite.
Work is capped at depth 32 and 65,535 subdivisions; violations are reported as
typed `FlatteningError` values.

## Development

```sh
swift test --enable-code-coverage
```

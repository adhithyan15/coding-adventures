# Changelog

## Unreleased

- Replace 100-point sampled center-arc bounds with analytic rotated extrema across wrapped signed sweeps; consume four neutral bounds records.
- Limit direct center-form cubic conversion to one finite turn, preserve zero/full-turn segment counts, and consume three neutral cubic records.
- Reject invalid or non-finite center inputs, evaluation parameters and derived output with recoverable .NET exceptions while preserving public signatures.
- Degenerate SVG endpoint arcs now evaluate on the endpoint line and return ordered bounds; center conversion remains optional and cubics remain empty.
- Reconcile the strict G2D03 radius and squared-endpoint guards and consume all four shared geometry fixtures.

## 0.1.0

- Add F# center-form and SVG endpoint-form elliptical arcs with cubic Bezier conversion.

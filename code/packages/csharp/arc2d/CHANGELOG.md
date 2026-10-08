# Changelog

## Unreleased

- Replace sampled center-arc bounds with analytic rotated extrema across wrapped signed sweeps; consume four neutral bounds records.
- Bound direct center-form cubic conversion to one finite turn, preserve zero/full-turn segment counts, and consume three neutral cubic records.
- Fail explicitly on invalid or non-finite center inputs, evaluation parameters and derived points before allocating; retain existing value-returning APIs.
- Degenerate SVG endpoint arcs now evaluate on the endpoint line and return ordered bounds instead of a missing result; center conversion remains nullable and cubics remain empty.
- Reconcile the strict G2D03 radius and squared-endpoint guards and consume all four shared geometry fixtures.

## 0.1.0

- Add C# center-form and SVG endpoint-form elliptical arcs with cubic Bezier conversion.

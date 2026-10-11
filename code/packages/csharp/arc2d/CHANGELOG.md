# Changelog

## Unreleased

- Degenerate SVG endpoint arcs now evaluate on the endpoint line and return ordered bounds instead of a missing result; center conversion remains nullable and cubics remain empty.
- Reconcile the strict G2D03 radius and squared-endpoint guards and consume all four shared geometry fixtures.

## 0.1.0

- Add C# center-form and SVG endpoint-form elliptical arcs with cubic Bezier conversion.

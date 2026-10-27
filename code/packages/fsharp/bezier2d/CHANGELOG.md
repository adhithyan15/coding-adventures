# Changelog

## Unreleased

- Align polyline flattening with the shared G2D02 corpus using bounded,
  iterative subdivision and finite-segment error measurement.
- Reject invalid tolerances, non-finite controls, derived non-finite geometry,
  and exhausted depth or subdivision budgets without returning partial output.
- Enforce a clean-checkout-safe 95% line-coverage gate on every build front.

## 0.1.0

- Add F# quadratic and cubic Bezier curves with evaluation, derivatives, splitting, polyline approximation, bounding boxes, and quadratic degree elevation.

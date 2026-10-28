# Changelog

## Unreleased

- Adopt the nine-case G2D02 safe-flattening corpus with finite-segment bounds,
  iterative depth/work budgets, and fail-closed numeric validation.
- Load the shared fixture dynamically on both native build fronts, provisioning
  pinned `dkjson` only for the test harness.

## [0.1.0] - 2026-04-02

### Added
- new_quad, eval_quad, deriv_quad, split_quad, polyline_quad, bbox_quad, elevate_quad
- new_cubic, eval_cubic, deriv_cubic, split_cubic, polyline_cubic, bbox_cubic

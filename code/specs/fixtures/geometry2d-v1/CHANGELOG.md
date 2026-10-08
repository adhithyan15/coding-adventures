# Changelog

## Unreleased

- Add eight closed G2D01/G2D02 records for noncommutative affine composition,
  invertible and threshold-singular matrices, vector translation exclusion,
  quadratic/cubic evaluation and derivatives, de Casteljau split controls,
  and tight extrema-based bounds (including cubic x overshoot).
- Extend the independent validator and negative mutation tests while retaining
  the version-1 transport and the original eight point/arc cases.

## 1.0.0 — 2026-10-07

- Pin normalization values below and at `1e-12`, with exact origin comparison
  so a returned sub-threshold input cannot pass by output tolerance alone.
- Pin degenerate SVG arc line evaluation and ordered endpoint bounds for
  zero, negative near-zero, near-coincident, and coincident inputs.

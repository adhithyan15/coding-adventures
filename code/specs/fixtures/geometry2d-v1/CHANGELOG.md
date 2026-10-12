# Changelog

## Unreleased

- Add four closed center-form tangent records for the raw normalized-parameter
  derivative, signed sweep, rotation and zero sweep. Correct the ellipse
  orthogonality requirement to use the rotated implicit gradient. Keep native
  adoption separately owned and preserve all 23 earlier records.
- Add a derivative-specific `1e7` teaching output bound while retaining the
  existing `1e6` point/Bezier cap, v1 transport, oracle `1e-15` and native `1e-12`
  absolute tolerances. Reject oversized integers before float conversion and
  reject non-finite or out-of-bound derived results before comparison.
- Exercise component mutations, wrong normalization/sign/rotation/translation,
  full-turn output bounds, closed shapes, non-finite values and enormous integers
  in the independent validator tests.

- Schedule the neutral geometry validator and its focused tests in the
  unconditional repo-wide metadata-contracts CI step, alongside the separate
  Bezier flattening oracle. Native reader adoption remains separately owned.
- Add seven closed G2D03 center-form cases: positive/negative wrapped analytic
  extrema, zero-sweep point bounds, rotated interior extrema, zero/full-turn
  cubic segment counts, and over-turn fail-stop. The validator derives extrema
  from coordinate derivatives and directed modulo-angle membership, not native outputs or
  fixed-step sampling. The v1 transport and tolerance remain unchanged.
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

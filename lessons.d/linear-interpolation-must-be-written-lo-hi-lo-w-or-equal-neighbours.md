---
category: Testing & coverage
---

# Linear interpolation must be written lo + (hi - lo) * w, or equal neighbours round outside their own range

**What went wrong.** `test-bootstrap`'s `percentile_sorted` interpolated as
`lo * (1 - w) + hi * w`. On paper that equals `lo` when `lo == hi`. In floating
point the two products round separately and can sum to one ulp above `lo`
(236322.0 at w = 0.225 gives 236322.00000000003). A coarse clock (macOS CI)
produces many identical benchmark samples, so the 97.5th percentile of the
bootstrap medians landed above the run's own maximum, and a shape test
(`min <= ci_lower && ci_upper <= max`) failed. The failure appeared on an
unrelated spec-only PR that happened to rebuild the crate.

**Fix.** Interpolate as `(lo + (hi - lo) * w).clamp(lo, hi)`. The subtraction
form returns `lo` exactly when `hi == lo`, and the clamp makes "between the two
neighbours" hold for every weight. Pin a few inputs that rounded upward under
the old form, and sweep many magnitudes and weights.

**Do differently.** Any statistic that promises an order or a range
(quantile chains, confidence intervals inside `[min, max]`) must be computed in
a form that keeps that promise in floating point, not only algebraically.
Test it with equal and near-equal neighbours, which are common on coarse
clocks. A timing test that "passes on Linux" is not evidence that its
arithmetic is sound.

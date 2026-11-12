## 0.5.2 — percentiles never leave their neighbouring samples

### Fixed: an impossible confidence interval on coarse clocks

- `percentile_sorted` interpolated as `lo * (1 - w) + hi * w`. With
  `lo == hi`, the two separately rounded products can sum to one ulp above
  `lo` (for example 236322.0 at weight 0.225 gives 236322.00000000003). A
  coarse timer produces many identical samples, so the 97.5th percentile of
  the bootstrap medians could come out one ulp above the run's own maximum.
  `every_run_has_ordered_quantiles_and_honours_its_bounds` failed on a macOS
  runner because of it (CI, PR #19204).
- The interpolation is now `lo + (hi - lo) * w`, clamped to `[lo, hi]`. It
  returns `lo` exactly when the neighbours are equal, and no percentile can
  leave the closed interval of its two neighbours. So the quantile chain stays
  ordered and the 95% CI stays inside `[min, max]`. This is still the VER07
  §3.3 linear interpolation; only the floating-point evaluation order changed.
- New tests: five pinned inputs that rounded upward under the old formula,
  and a 2,000-step sweep that checks every percentile stays between its
  neighbours.

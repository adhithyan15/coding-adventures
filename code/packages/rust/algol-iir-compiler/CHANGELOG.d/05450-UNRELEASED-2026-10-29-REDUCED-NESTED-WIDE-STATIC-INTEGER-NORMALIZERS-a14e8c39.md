## 0.545.0 - 2026-10-29

### Added

- Preserve exact checked integer `div` and `mod` between nested built-in
  `abs`, `sign`, and `entier` calls until a following reduction restores the
  exact binary64 widening range.

## 0.541.0 - 2026-10-28

### Added

- Preserve exact checked `i64` roots above `2^53` through pure nested chains
  of built-in `abs`, `sign`, and `entier` when a following integer reduction
  restores the exact widening range.

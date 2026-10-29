## 0.542.0 - 2026-10-28

### Added

- Preserve exact checked `i64` roots above `2^53` through unary signs between
  nested built-in `abs`, `sign`, and `entier` calls when a following integer
  reduction restores the exact widening range.

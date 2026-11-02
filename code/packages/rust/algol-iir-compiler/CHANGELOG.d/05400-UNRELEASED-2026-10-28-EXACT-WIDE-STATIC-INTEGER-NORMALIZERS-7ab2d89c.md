## 0.540.0 - 2026-10-28

### Added

- Preserve exact checked `i64` static integer arguments above `2^53` through
  call-free built-in `abs`, `sign`, and `entier` when an enclosing integer
  reduction brings the result back into the exact widening range.

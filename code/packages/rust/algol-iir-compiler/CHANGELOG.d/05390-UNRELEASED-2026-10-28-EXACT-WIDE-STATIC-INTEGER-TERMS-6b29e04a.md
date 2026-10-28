## 0.539.0 - 2026-10-28

### Added

- Preserve exact checked `i64` static terms above `2^53` inside sign-rooted
  integer reductions that use `div` or `mod` before widening. Unreduced terms,
  early real division, and overflow remain rejected.

## 0.534.0 - 2026-10-25

### Added

- Preserve the exact common divisor when `div` or `mod` reduces a stable
  sign-rooted value whose magnitude equals its divisor bound. The sharper
  proof composes across stable multiplied sign roots and can establish that a
  later real `/` remains integral; broader value sets still fail closed.

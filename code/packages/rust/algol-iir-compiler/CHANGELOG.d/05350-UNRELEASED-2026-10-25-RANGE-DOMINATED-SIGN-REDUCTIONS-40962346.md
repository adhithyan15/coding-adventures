## 0.535.0 - 2026-10-25

### Added

- Classify sign-rooted `div` as exact zero and `mod` as an identity when the
  complete magnitude is below the static divisor. The resulting common-divisor
  proof composes through multiplied roots and later exact real division while
  zero and dynamic divisors remain excluded.

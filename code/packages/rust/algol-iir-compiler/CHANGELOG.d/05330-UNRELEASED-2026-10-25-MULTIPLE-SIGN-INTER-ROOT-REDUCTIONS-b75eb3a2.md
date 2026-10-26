## 0.533.0 - 2026-10-25

### Added

- Extend the conservative exact-integer proof for multiple dynamic `sign`
  products to ordered checked static `div`, `mod`, and exact `/` reductions
  between roots when each later root enters by multiplication. Zero divisors,
  inexact divisions, dynamic or rooted divisors, and non-multiplying roots
  still fail closed.

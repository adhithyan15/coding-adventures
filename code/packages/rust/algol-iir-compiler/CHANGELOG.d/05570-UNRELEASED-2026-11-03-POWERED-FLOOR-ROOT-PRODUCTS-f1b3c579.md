## 0.557.0 (UNRELEASED) - 2026-11-03

### Added
- Reduce bounded nonnegative powers of a static square root to an exact integer coefficient times at most one remaining root, preserving checked products and later static coefficient divisions before `entier` without binary64 widening.
- Reject dynamic or negative exponents, exponent chains beyond the existing unroll cap, checked coefficient overflow, and powered root divisors conservatively.

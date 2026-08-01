# Changelog

## 1.1.0 - 2026-08-01

- Add the minimum-positive-subnormal frequency boundary, including its
  positive-infinite represented period and finite amplitude-bounded
  evaluation contract.
- Make phase reduction and infinite-period handling explicit in the shared
  reference calculation.

## 1.0.0 - 2026-07-31

- Define the closed Draft 2020-12 schema and tagged binary64 scalar encoding.
- Add PHY00 constants, finite precision, poles, quadrants, signed zero,
  tiny/subnormal/maximum square roots, infinity, NaN, and validation cases.
- Add PHY01 construction, derived values, periodicity, validation,
  zero-amplitude, non-finite-time, and extreme finite evaluation cases.
- Add binary64 overflow, underflow, and maximum-tolerance semantic gates.
- Add an exact canonical Dart representation so package consumers need no
  runtime filesystem authority and cannot disagree with the strict JSON load.

## 0.544.0 - 2026-10-29

### Added

- Preserve exact checked `i64` powers between nested built-in `abs`, `sign`,
  and `entier` calls until a following integer reduction restores the exact
  binary64 widening range. Exponents remain bounded by the existing 4,096 cap.

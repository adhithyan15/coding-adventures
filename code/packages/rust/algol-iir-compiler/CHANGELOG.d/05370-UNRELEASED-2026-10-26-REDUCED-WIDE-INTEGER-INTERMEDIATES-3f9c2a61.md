## 0.537.0 - 2026-10-26

### Added

- Preserve exact sign-rooted integer proofs through multiplication above
  binary64's contiguous exact range when the intermediate remains within
  `i64` and a later static `div` or `mod` reduces it before widening. Overflow
  and over-wide real division continue to fail closed.

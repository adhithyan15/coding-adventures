## 0.543.0 - 2026-10-29

### Added

- Preserve exact checked `i64` arithmetic through `+`, `-`, and `*` between
  nested built-in `abs`, `sign`, and `entier` calls until a following integer
  reduction restores the exact binary64 widening range.

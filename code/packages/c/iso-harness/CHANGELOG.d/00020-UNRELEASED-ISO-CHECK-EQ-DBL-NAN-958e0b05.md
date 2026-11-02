## [Unreleased] — ISO_CHECK_EQ_DBL no longer passes on NaN

### Fixed

- **`ISO_CHECK_EQ_DBL` passed whenever the actual value, expected value or
  tolerance was NaN.** It failed only when `|a - b| > eps`, and every
  comparison with NaN is false, so a computation that produced NaN satisfied
  the check against any expected value. It now passes only when `a == b`
  (which also covers equal infinities, whose difference is NaN), when both
  values are NaN, or when `|a - b| <= eps`. `inf` vs `-inf` now fails too.

### Added

- `selftest/falsifiable.c` and `.cpp`: feed every check a value it must
  reject and exit 0 only when exactly those checks failed. Both run in
  `selftest.sh` and `selftest.ps1` on every compiler. Against the old header
  the fixture sees 9 of its 13 expected failures and fails.

### Changed

- The changelog is now sharded (`CHANGELOG.d/`).

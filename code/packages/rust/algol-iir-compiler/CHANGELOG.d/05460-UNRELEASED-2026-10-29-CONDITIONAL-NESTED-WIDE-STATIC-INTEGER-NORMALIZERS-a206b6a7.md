## 0.546.0 - 2026-10-29

### Added

- Preserve exact conditional expressions between nested built-in `abs`,
  `sign`, and `entier` calls when the selector is side-effect-free and static
  or every branch proves the same checked integer result.

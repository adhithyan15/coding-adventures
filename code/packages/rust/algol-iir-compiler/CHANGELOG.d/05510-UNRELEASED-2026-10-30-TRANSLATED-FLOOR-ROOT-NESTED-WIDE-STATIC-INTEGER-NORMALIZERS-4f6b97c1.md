## 0.551.0 - 2026-10-30

### Added

- Preserve checked integer translations around one exact static square root
  through `entier`, including the negated ceiling-root adjustment required by
  subtraction, without routing wide radicands through binary64.

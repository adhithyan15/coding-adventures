## 0.550.0 - 2026-10-30

### Added

- Preserve exact floor semantics for built-in `entier(-sqrt(n))` between
  nested integer normalizers, including non-square radicands beyond binary64's
  exact integer range.

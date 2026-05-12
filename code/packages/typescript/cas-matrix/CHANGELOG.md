# Changelog

## Unreleased

- Route concrete numeric TypeScript matrix arithmetic through the shared
  `matrix` backend dispatch point while preserving symbolic and exact-rational
  fallback behavior.

## 0.1.0

- Port matrix construction, shape helpers, arithmetic, determinant, and inverse
  to pure TypeScript.
- Add exact rational norms, LU decomposition with partial pivoting, and
  nullspace/columnspace/rowspace basis operations.

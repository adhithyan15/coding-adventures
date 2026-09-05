# Changelog

## Unreleased

- Run POSIX dependency lock commands explicitly through Bash on Windows.

- Route concrete numeric TypeScript matrix arithmetic through the shared
  `matrix` backend dispatch point while preserving symbolic and exact-rational
  fallback behavior.
- **MX08 Phase 3 (verification)**: on Node, the `matrix` package's
  `CpuMatrixBackend` now delegates to the Rust `matrix-cpu` executor via
  `@coding-adventures/matrix-rust-napi` (MX08 Phase 2, PR #3571).  The
  concrete-number fast path in `cas-matrix` picks up the speedup
  transparently — **no source change in this package**.  Verified: all
  34 tests pass after the MX08 Phase 2 refactor.  Browser builds keep
  the pure-TS implementation per the new package.json `exports`
  conditional.

## 0.1.0

- Port matrix construction, shape helpers, arithmetic, determinant, and inverse
  to pure TypeScript.
- Add exact rational norms, LU decomposition with partial pivoting, and
  nullspace/columnspace/rowspace basis operations.

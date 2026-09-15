## 0.1.20 — 2026-08-15 — baseline regen: blocktype fix (task #81)

### Changed

- Baseline regen following `wasm-execution` 0.9.1 / `wasm-validator`
  0.2.6 (task #81 -- `v128`/`funcref`/`externref` single-value blocktypes
  were being misdecoded as bogus negative type indices). `simd_const.wast`
  moves from `module: 307/309, assert_return: 189/240` to `module:
  308/309, assert_return: 209/240` -- the module declaring `(block
  (result v128) ...)`-shaped functions (a SEPARATE module from the
  still-open v128-global-initializer trap, task #79) now validates and
  instantiates for real, and the ~20 collateral "no module registered"
  failures its own callers previously hit are gone. Confirmed via a full
  JSON diff that this is the ONLY file affected; zero regressions
  elsewhere.


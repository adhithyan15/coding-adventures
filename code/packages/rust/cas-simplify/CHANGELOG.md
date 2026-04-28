# Changelog — cas-simplify (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-simplify` package.
- `canonical(node: IRNode) -> IRNode` — structural normalization pass:
  - Flatten nested `Add`/`Mul` into flat argument lists.
  - Sort commutative args by a stable rank + display-string key
    (Integer < Rational < Float < Symbol < Apply < Str).
  - Singleton drop: `Add(x)` → `x`, `Mul(x)` → `x`.
  - Empty container: `Add()` → `0`, `Mul()` → `1`.
  - Idempotent: `canonical(canonical(x)) == canonical(x)`.
- `numeric_fold(node: IRNode) -> IRNode` — constant-folding pass:
  - Folds all adjacent numeric literals in `Add`/`Mul` arg lists into one.
  - Exact rational arithmetic via `i128` intermediaries (overflow-safe).
  - Float contamination: one `Float` in a cluster promotes the whole fold to `f64`.
  - Identity literals dropped when non-literal args remain.
- `build_identity_rules() -> Vec<IRNode>` — algebraic identity rule list built with
  `cas-pattern-matching` primitives.  Rules cover: add/mul identity, zero
  product, power identities, self-cancellation, log/exp inverses, trig at zero.
- `simplify(expr: IRNode, max_iterations: usize) -> IRNode` — fixed-point
  pipeline: `canonical → numeric_fold → rewrite(IDENTITY_RULES)` repeated until
  stable or iteration bound reached.
- 43 integration tests + 2 doc-tests; all passing.

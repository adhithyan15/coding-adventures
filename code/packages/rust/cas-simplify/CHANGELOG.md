# Changelog — cas-simplify (Rust)

## [0.2.0] — 2026-05-29

**Track G2 — compound-relation assumption store (Rust port).**

Extends `AssumptionContext` so `assume_relation(...)` and
`is_true_relation(...)` accept arbitrary relational shapes, not just
plain-symbol-vs-zero.  Previously `assume(a^2 > b^2)` was silently
dropped; under Track G2 it is canonicalised into a
`(IRNode, &'static str, IRNode)` triple and stored in a new
`HashSet`.  Subsequent `is(a^2 > b^2)` / `is(b^2 < a^2)` queries
return `Some(true)` via structural lookup with commutativity-aware
rewriting.  The legacy plain-symbol path is unchanged; the new path
fires only when the plain-symbol path returns `None`.

Mirrors Python `cas-simplify` 0.4.0 (Track G1) and TypeScript
`@coding-adventures/cas-simplify` 0.2.0.  The symbolic-coefficient
Weierstrass integrator that consumes this store ships in
`symbolic-vm` 0.19.0.

### Added

- Private `general_relations` field on `AssumptionContext`, a
  `HashSet<(IRNode, &'static str, IRNode)>` of canonical compound
  relations.
- Private helpers `parse_relation`, `canon_relation`, `node_key`, and
  the centralised `head_to_op` map.
- `assume_relation`, `forget_relation`, and `is_true_relation` now
  have a compound-relation fallback when the plain-symbol path
  doesn't apply.  `forget_all` clears both stores.

### Semantics

- No negative-knowledge inference: `assume(a^2 > b^2)` does NOT make
  `is(a^2 < b^2)` return `Some(false)` — it returns `None`.
- Commutativity is honoured: `is(b^2 < a^2)` ≡ `is(a^2 > b^2)`,
  `is(b^2 = a^2)` ≡ `is(a^2 = b^2)`, and similarly for `<=` / `>=` and `!=`.

## [0.3.0] — 2026-05-29

### Added

- Released the previously-Unreleased deterministic
  `AssumptionContext::facts_for(...)` and
  `AssumptionContext::symbols_with_facts()` metadata queries as part
  of the `macsyma-truly-finish-plan` closure sweep (Track N).

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

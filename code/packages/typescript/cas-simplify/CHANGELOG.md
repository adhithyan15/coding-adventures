# Changelog

## [0.2.0] — 2026-05-29

**Track G2 — compound-relation assumption store (TypeScript port).**

Extends `AssumptionContext` so `assumeRelation(...)` and
`isTrueRelation(...)` accept arbitrary relational shapes, not just
plain-symbol-vs-zero.  Previously `assume(a^2 > b^2)` was silently
dropped; under Track G2 it is canonicalised into a `(lhs, op, rhs)`
triple and stored in a new structural-key set.  Subsequent
`is(a^2 > b^2)` / `is(b^2 < a^2)` queries return `true` via lookup
with commutativity-aware rewriting.  The legacy plain-symbol path is
unchanged; the new path fires only when the plain-symbol path
returns `undefined`.

Mirrors Python `cas-simplify` 0.4.0 (Track G1).  The
symbolic-coefficient Weierstrass integrator that consumes this
compound-relation store ships in `symbolic-vm` 0.19.0.

### Added

- Private `generalRelations` set of canonical-key strings for
  compound relations.
- `assumeRelation`, `forgetRelation`, and `isTrueRelation` now have
  a compound-relation fallback when the plain-symbol path doesn't
  apply.  `forgetAll` clears both stores.

### Semantics

- No negative-knowledge inference: `assume(a^2 > b^2)` does NOT make
  `is(a^2 < b^2)` return `false` — it returns `undefined`.
- Commutativity is honoured:
    - `is(b^2 < a^2)` ≡ `is(a^2 > b^2)`,
    - `is(b^2 = a^2)` ≡ `is(a^2 = b^2)`,
    - and the same for `<=`/`>=`, `!=`.

## [0.3.0] — 2026-05-29

- Released the previously-Unreleased deterministic
  `AssumptionContext.factsFor(...)` and
  `AssumptionContext.symbolsWithFacts()` metadata queries as part of
  the `macsyma-truly-finish-plan` closure sweep (Track N).

## 0.1.0

- Add canonicalization, numeric folding, and fixed-point simplification.

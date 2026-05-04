# Changelog — constraint-core

## [0.1.0] — 2026-04-30

Initial release.  **LANG24 PR 24-A** — predicate AST + sort/logic/theory
enums + normalisation passes for the generic Constraint-VM.

### Added

- `Predicate` enum (24 variants, `#[non_exhaustive]`) — recursive
  constraint-language AST covering boolean logic (`Bool`, `And`, `Or`, `Not`,
  `Implies`, `Iff`), variables and apply (`Var`, `Apply`), arithmetic literals
  (`Int`, `Real`), arithmetic ops (`Add`, `Sub`, `Mul`), comparisons (`Eq`,
  `NEq`, `Le`, `Lt`, `Ge`, `Gt`), conditional (`Ite`), quantifiers (`Forall`,
  `Exists`), and array operations (`Select`, `Store`).

- `Sort` enum — `Bool`, `Int`, `Real`, `BitVec(width)`,
  `Array { idx, val }`, `Uninterpreted(name)`.  `#[non_exhaustive]`.

- `Logic` enum — `QF_Bool`, `QF_LIA`, `QF_LRA`, `QF_BV`, `QF_AUFLIA`, `LIA`,
  `ALL`.  Declares which theories a constraint program uses so an engine can
  reject programs whose logic isn't supported up-front.  `#[non_exhaustive]`.

- `Theory` enum — `Bool`, `LIA`, `LRA`, `Arrays`, `BitVectors`, `EUF`,
  `Strings`, `NRA`, `FP`.  Tactics declare their `Theory`s; engines compose
  them via Nelson-Oppen.  `#[non_exhaustive]`.

- `Rational { num: i128, den: i128 }` — hand-rolled GCD-reduced rational for
  the `Real` literal variant.  Panics on zero denominator at construction;
  normalises sign so denominator is always positive.

- Smart constructors:
  - `Predicate::and(parts)` — drops `Bool(true)` operands, short-circuits on
    `Bool(false)`, flattens nested `And`, unwraps singletons, returns
    `Bool(true)` for empty input.
  - `Predicate::or(parts)` — mirror of the above (drops `Bool(false)`,
    short-circuits on `Bool(true)`, returns `Bool(false)` for empty).
  - `Predicate::not(p)` — folds `Not(Bool(b))` and eliminates `Not(Not(p))`.

- Normalisation passes:
  - `Predicate::to_nnf()` — negation normal form.  De Morgan + double-neg
    elim + atomic-comparison negation.  Desugars `Implies` and `Iff` first.
  - `Predicate::to_cnf()` — conjunctive normal form.  `to_nnf()` then naive
    distribution of `Or` over `And` (exponential worst case; acceptable for
    small refinement-type predicates).
  - `Predicate::simplify()` — folds `Ite` with constant conditions and
    deduplicates `And`/`Or` operands.

- `Predicate::free_vars()` — returns `BTreeSet<String>` of free-variable
  names; quantifier scopes are honoured (bound names removed inside the
  quantifier body).

- `infer_sort(predicate, env)` + `SortEnv = HashMap<String, Sort>` +
  `SortError` enum — sort inference with typed errors (`Mismatch`,
  `UnknownVar`, `ArityMismatch`, `NonBoolBranch`, …).

- `Display` impl on `Predicate`, `Sort`, `Logic`, `Rational` producing
  Lisp-style s-expressions — debugging aid and basis for the eventual
  SMT-LIB exporter.

- 42 unit tests covering all of the above (smart constructors, normalisation,
  free-vars, sort inference positive/negative, `Rational` GCD reduction,
  `i128::MIN` rejection at `Rational` boundaries, `Display` round-trips).

### Notes

- Pure data + algorithms.  Zero dependencies.  No I/O, no solver state, no
  filesystem/network/process/env access.  See `required_capabilities.json`.
- All public enums are `#[non_exhaustive]` so v2/v3 theories can plug in
  without breaking downstream matchers.
- **Caller responsibilities** (documented in the crate-level docs): predicate
  depth and CNF blow-up are not bounded at this layer — engines ingesting
  untrusted predicates must enforce limits at the boundary.  `Rational::new`
  rejects `i128::MIN` for either operand to avoid negation-overflow UB.

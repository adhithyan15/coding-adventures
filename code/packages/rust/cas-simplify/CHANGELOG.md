# Changelog — cas-simplify (Rust)

## [0.5.0] — 2026-07-16

### Added

- **`collect_terms` — `expand()` now collects like terms.** Previously an
  honestly-documented, explicitly-tracked gap (see `expand`'s own module
  docs prior to this release, and the `spice-macsyma-pending-work.md`
  `Expand` entry): `expand((x+1)^2)` returned the raw, uncollected
  `1 + x + x + x*x` rather than `1 + 2*x + x^2`. New `collect_terms` module:
  flattens an `Add`/`Sub` subtree into signed terms (descending through the
  nested `Add(Add(..), Add(..))` shape square-and-multiply's intermediate
  results leave behind), decomposes each term into a `(coefficient,
  monomial)` pair — reusing `numeric_fold`'s exact-rational `Acc`
  accumulator so both passes share the same GCD-reduced,
  float-contamination-aware arithmetic — groups by monomial (summing
  coefficients, dropping exact-zero groups, so genuine cancellations like
  the cross terms in `(a+b)*(a-b)` actually disappear), and rebuilds. Also
  folds repeated multiplication into a power (`x*x` → `x^2`) as a
  byproduct of the same monomial decomposition — the *other* half of the
  gap `expand`'s docs called out. `expand()` now runs `collect_terms` on
  the raw distribution before the final `simplify` pass.
- Flattens nested `Mul` structure first (`expand_mul` only ever wraps two
  operands per call and never re-flattens against an already-`Mul`
  operand — square-and-multiply routinely leaves `Mul(Mul(a, a), a)`
  behind for `a^3`, not the flat `Mul(a, a, a)` the base decomposition
  needs to see all three factors as the same base) — found by this
  package's own test suite (`expand_pow_of_trinomial_multivariate`'s (a+b)³
  case) before ever reaching `/security-review`.
- `O(n log n)` grouping (sort-then-merge-adjacent), not `O(n²)`
  (find-or-insert) — matters at `EXPAND_MAX_TERMS` (10,000) scale; see the
  module's own DoS-safety note for why this pass can't reopen the growth
  its sibling guard already closed.
- Updated the two downstream consumers (`macsyma-runtime`'s `expand`,
  `wolfram-runtime`'s `Expand[...]`) — both delegate to this crate's
  `expand` unchanged, so they inherit collected output automatically; their
  own exact-output tests and doc comments (which pinned/described the old
  uncollected shape) are updated in lockstep.
- 30 tests total (up from 24), including adversarial cases (opposite-signed
  cancellation to exact zero, exact-rational coefficient summation via the
  shared `Acc`, negative-exponent bases, an opaque non-`Pow` repeated
  factor like `Sin(x)*Sin(x)`).

## [0.4.1] — 2026-07-12

### Fixed

- **`term_count` was blind to large subtrees hidden under `Div`/`Neg`/every
  transcendental wrapper** (`Sin`, `Log`, `Exp`, ...) and un-distributed
  `Pow`. `expand_apply` recursively expands the *children* of these heads
  but never distributes the wrapper itself, so `Div(huge_expanded_tree, y)`
  is an entirely ordinary shape a real expansion produces — but
  `term_count` previously treated any such node as size `1` (the same
  `_ => 1` catch-all a prior fix already closed for refused `Mul` nodes).
  If a `Div`/`Neg`/transcendental-wrapped subtree later became an operand
  under a further `Add`-distribution, `expand_mul` would clone the whole
  hidden subtree once per term of the other side — real cost proportional
  to its true size, invisible to the cap check that saw only "1". A
  9,000-term `Div`-wrapped subtree multiplied by an ordinary 20-term sum
  reproducibly reached 180,000+ nodes under the old logic (empirically
  confirmed via the same disable-and-reproduce methodology used for the
  original `Mul`-blindness fix) despite `EXPAND_MAX_TERMS` (10,000) being
  configured — the cap check saw `1 * 20 = 20`, nowhere near the limit.
- Fixed by generalizing `term_count`'s fallback: any `Apply` node whose
  head is not `Mul` (which multiplies its children's counts) now sums its
  children's term counts — the same measure `Add`/`Sub` already used,
  extended to every wrapper shape `expand_apply` can leave in place, not
  just `Add`/`Sub` specifically. 2 new regression tests: an adversarial
  `Div`-wrapped-subtree reproduction (verified to fail without the fix,
  restored to confirm it passes with it) and a correctness check that
  `Neg`/`Sin` wrappers are sized by their contents, not treated as `1`.
- **Follow-up finding from `/security-review`, fixed in the same PR**: the
  new fallback's summation used plain `Iterator::sum::<usize>()`, unlike
  the sibling `Mul` arm's `saturating_mul`. Since a `Mul` subtree can
  legitimately saturate `term_count` to `usize::MAX` from a modest,
  ordinary tree, summing that value with any sibling would either panic
  (overflow-checked debug/test builds — confirmed by reverting the fix
  and reproducing the exact panic) or silently wrap to a small value
  (release builds) — reintroducing the guard's exact blindness this PR
  exists to close, just via arithmetic overflow instead of a missing
  match arm. Fixed with `saturating_add`, plus a new regression test
  sized to actually reach the saturation boundary (70 chained two-term
  factors, 2^70).

## [0.4.0] — 2026-07-03

### Added

- `expand(node: IRNode) -> IRNode` — full polynomial expansion: distributes
  `Mul` over `Add`/`Sub` and expands bounded non-negative integer `Pow`s via
  square-and-multiply, then cleans up the result through the existing
  `simplify` pipeline. A faithful port of the Python reference's general
  recursive-distributor path (`symbolic_vm.cas_handlers._sym_expand`),
  generalized to the n-ary `Add`/`Mul` shape this Rust IR actually produces.
- `EXPAND_MAX_POW` (32) and `EXPAND_MAX_TERMS` (10,000) — DoS guards.
  Square-and-multiply on a multi-term base squares the term count at every
  squaring step (doubly exponential in the number of squarings, not the
  exponent), so `EXPAND_MAX_TERMS` refuses any single distribution step
  whose *product* of operand term-counts would exceed the cap, checked
  before allocating rather than after.
- **Honest scope note**: `expand` does not collect like terms — repeated
  monomials produced by distribution are not merged and combined coefficients
  are not summed (e.g. `expand((x+1)^2)` returns `1 + x + x + x*x`, not the
  fully-collected `1 + 2*x + x^2`). Mathematically correct, not maximally
  consolidated. See the module docs for why.

This closes a real, previously-undocumented gap: no consumer had a working
`Expand`/`expand()` handler at all — `symbolic-vm`'s shared handler table
never registered one, so Macsyma's `expand(...)` silently returned its input
unevaluated. See `macsyma-runtime` 0.6.0 and `spice-macsyma-pending-work.md`.

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

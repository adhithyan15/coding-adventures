# Changelog — symbolic-vm (Rust)

## [0.20.2] — 2026-07-16

### Changed

- `handlers::derivative_handler`'s differentiate-then-simplify logic is now
  a `pub` free function, `handlers::differentiate(vm, f, x)`. No behavior
  change for `derivative_handler` itself (it now just calls the extracted
  function) — this only widens visibility so other language runtimes
  sharing this crate's `VM`/`IRNode` types can reuse the exact same
  differentiation pipeline Macsyma's own `D` already runs, rather than
  reimplementing or duplicating it. Unlike `factor_handler` (made `pub`
  directly, since it already did its own arity check), `derivative_handler`
  itself still panics on the wrong argument count — a real internal
  invariant for this crate's own dispatch table — so callers with a
  fail-soft contract (leave the form unevaluated instead of panicking)
  validate arity themselves and call `differentiate` with the unpacked
  `f`/`x` instead of the whole `IRApply`. First consumer: `wolfram-runtime`'s
  `D[expr, x]` wiring (W-22, see that crate's own changelog).

## [0.20.1] — 2026-07-12

### Changed

- `handlers::factor_handler` is now `pub` (was module-private). No
  behavior change — this only widens visibility so other language
  runtimes sharing this crate's `VM`/`IRApply` types can call the exact
  same `Factor` evaluation pipeline Macsyma's own runtime already uses,
  rather than reimplementing or duplicating it. First consumer:
  `wolfram-runtime`'s `Factor[...]` wiring (W-22, see that crate's own
  changelog).

## [0.20.0] — 2026-05-29

**Track K2 — n-variate Hensel factor bridge (Rust port).**

Wires the new `try_n_variate_hensel` from cas-factor 0.3.0 into the
`Factor(...)` IR handler.  Mirrors the Python Track K1 bridge in
`symbolic-vm/cas_handlers.py` (PR #5590) and the TS port in
`@coding-adventures/symbolic-vm` 0.20.0.

Algorithm (n ≥ 3, generic — not per-arity):

1. Identify all free variables in the input (`find_n_variables`,
   bounded at 8 distinct symbols so a pathological input can't
   allocate gigantic sparse-dict keys).
2. Convert to an `NPoly` via `ir_to_npoly`.  Returns `None` for
   floats, foreign symbols, transcendentals (Sin/Log/…), or non-integer
   exponents.
3. Call `try_n_variate_hensel`.  On success, convert each factor back
   to IR via `npoly_to_ir` using **left-nested binary Add/Mul** (the
   primitive Add/Mul handlers are strictly binary, so n-ary Apply
   nodes with three or more children would crash).
4. Hook into `factor_handler` AFTER the bivariate Hensel path, BEFORE
   the unevaluated-wrapper fallback.

Catches `x³ + y³ + z³ − 3xyz = (x+y+z)(x²+y²+z²−xy−yz−zx)`,
`(x+y+z)(x+2y+3z) = x²+3xy+4xz+2y²+5yz+3z²`, and similar trivariate
cases.  Falls through cleanly on irreducibles and transcendentals.

### Added

- `try_n_variate_hensel_ir` — top-level IR glue mirroring Python
  `_try_n_variate_hensel_ir`.
- `find_n_variables`, `ir_to_npoly`, `npoly_to_ir`, `fold_binary`
  helpers mirroring `_find_n_variables`, `_ir_to_npoly`,
  `_npoly_to_ir`, and the left-nested-binary-fold convention.
- `tests/n_variate_factor.rs` — 6 end-to-end pipeline tests
  exercising `Factor(...)` over the VM: sum-of-cubes identity, linear
  product round-trip, irreducible fall-through, transcendental safety,
  bivariate regression, univariate regression.

### Changed

- `cas-factor` crate dependency reflected at the 0.3.0 floor
  (n-variate Hensel landed there).
- `factor_handler` dispatch order: univariate → bivariate Hensel →
  n-variate Hensel → unevaluated wrapper.

## [0.19.0] — 2026-05-29

**Track G2 — symbolic-coefficient Weierstrass lift (Rust port).**

Generalises the Phase-34/35/36/37 Weierstrass substitution
`∫ c / (a + b·trig(α·x + β)) dx` from concrete rational `a, b` to
symbolic ones.  When the numeric pattern returns `None` because
either coefficient is a free IR expression, the new helper consults
`vm.assumptions` for the sign of the discriminant `a² − b²` and,
upon finding a declared inequality / equality, emits the matching
arctan / log / degenerate closed form with symbolic
`Sqrt(a² − b²)` (or `Sqrt(b² − a²)`) in the result.  When no
assumption pins down the sign, the integral is left unevaluated.

This depends on the compound-relation extension to
`cas_simplify::AssumptionContext` shipped in cas-simplify 0.2.0
(same PR, Track G2).  Mirrors Python `symbolic-vm` 0.74.0 and
TypeScript `@coding-adventures/symbolic-vm` 0.19.0.

### Added

- New `assumptions: AssumptionContext` field on `VM` — populated via
  the `Assume(...)` / `Forget(...)` / `ForgetAll()` handlers and
  consulted by `try_weierstrass_symbolic_coefficients` via a
  thread-local snapshot.
- `Assume`, `Forget`, and `ForgetAll` handlers registered on the
  symbolic backend.  Both relational heads are added to the
  hold-evaluate set so the relation argument reaches the handler
  intact.
- `try_weierstrass_symbolic_coefficients` — symbolic dispatcher,
  invoked after the numeric helper returns `None`.
- `weierstrass_parse_a_plus_b_sincos_symbolic` — symbolic sibling of
  the numeric parser.
- Branch emitters `try_weierstrass_{arctan,log,degenerate}_symbolic`.
- New `cas-simplify` crate dependency.

### Regression

The numeric Weierstrass path is tried first and unchanged; the
symbolic path explicitly bails out when both `a` and `b` are
rational, so concrete-coefficient integrals continue to use the
arithmetic-folded numeric closed forms.  All 225 existing tests still
pass; 8 new tests cover the symbolic branches.

## [0.18.0] - 2026-06-06

### Added

- Port the Python Phase 23 `Exp(c*x^2)` integration fallback for exact
  rational, nonzero `c`, returning `Erf` for negative coefficients and
  `Erfi` for positive coefficients.

## [0.17.0] — 2026-06-06

### Added

- Port the Python Phase 23 Fresnel integration fallback for
  `Integrate(Sin(a*x^2), x)` / `Integrate(Cos(a*x^2), x)` and
  `q*%pi*x^2` variants into the Rust VM.
- Tighten the previous IBP fallthrough tests so `sin(x^2)` and `cos(x^2)`
  must now return `FresnelS` / `FresnelC` forms instead of accepting an
  unevaluated `Integrate(...)`.

## [0.16.0] — 2026-05-28

**Track E2 — generic tabular integration-by-parts fallback (Rust port).**
Mirrors the Python `ibp_tabular.py` reference (Track E1) and the
TypeScript port at `0.16.0`.  Closes the cross-language gap for the
`Integrate` handler.

When every shape-specific handler in `integrate` returned the original
unevaluated `Integrate(...)` form for a `Mul`-shaped integrand, the new
`try_ibp_tabular` fallback makes a last-ditch attempt by **generic
tabular IBP**:

```
For f = u(x) · w(x) with u polynomial in x:
  ∫ u·w dx = Σ_{k=0}^{N-1} (-1)^k · u^(k)(x) · I^(k+1)(w)
```

where N = deg(u) + 1.  The I-column entries `∫w, ∫∫w, ..., ∫^N w` come
from a recursive call to `integrate` (not the outer handler — this
avoids re-entering the IBP fallback during column construction); any
step that fails to close abandons the partition.  Bounded by
`IBP_MAX_FACTORS = 5` and `IBP_MAX_POLY_DEGREE = 8`.

### Added

- `try_ibp_tabular(f, x, vm)` — top-level fallback.  Returns the
  closed-form antiderivative or `None`.
- `ibp_flatten_mul(node)` — flattens nested-binary `Mul(a, Mul(b, c))`
  trees so the IBP search isn't fooled by parse-tree grouping.
- `ibp_multiply_ir(factors)` — rebuilds a left-associative `Mul` chain.
- `ibp_polynomial_degree(node, x)` — returns the polynomial degree in x
  (`Some(-1)` for zero, `None` for non-polynomial).
- `ibp_contains_integrate(node)`, `ibp_is_zero(node)`,
  `ibp_try_split(...)`, `ibp_combinations(n, k)` — implementation
  helpers.

### Changed

- `integrate_handler` now invokes `try_ibp_tabular` as the **last**
  fallback before returning the unevaluated `Integrate(...)` form.
  Closed-form results are passed through `vm.eval` for simplification.

### Test plan

Six tests in `tests/ibp_tabular.rs`:

1. `∫ x·sin(x) dx` closes via tabular IBP.
2. `∫ x²·eˣ dx` closes via tabular IBP.
3. `∫ x³·cos(x) dx` closes (verified against trapezoidal rule).
4. Fallthrough: `∫ 1/x dx → log(x)` (IBP short-circuits — head is DIV).
5. Fallthrough: `∫ sin(x²) dx` stays unevaluated or returns Fresnel —
   IBP fabricates no bogus elementary form.
6. Regression: `∫ cos(x²) dx` (Fresnel family) still stays unevaluated.

## [0.15.0] — 2026-05-28

**Track D2 — bivariate Hensel lifting in `Factor` (Rust port).**

Wires the new `cas-factor` 0.2.0 `try_bivariate_hensel` into the
`Factor` head's multivariate fall-through chain.  When none of the
existing pattern handlers (perfect square/cube, difference of squares,
cubic identity, grouping, common-factor) recognise the input, the
handler now converts the IR to `cas_factor::BiPoly`, calls
`try_bivariate_hensel`, and emits a `Mul(...)` of the lifted factors.
Mirrors the Python `_try_bivariate_hensel_ir` glue in
`symbolic-vm/cas_handlers.py`.

### Added

- `find_two_variables(node)` — walks the IR tree, returns the first two
  distinct free variable names or `None` (third variable, transcendental
  constant, etc. all disqualify).
- `ir_to_bipoly(node, x, y)` — converts the polynomial subset of IR
  (`Add`, `Sub`, `Mul`, `Pow`, `Neg`, `Integer`, `Rational`, symbol) to
  a sparse `cas_factor::BiPoly`.  Returns `None` for floats,
  transcendentals, non-integer or negative exponents, foreign symbols.
- `bipoly_to_ir(p, x, y)` — converts a `BiPoly` back to IR with
  deterministic descending-degree term order.
- `try_bivariate_hensel_ir(inner)` — the top-level glue invoked by
  `factor_handler`.

### Changed

- `factor_handler` — when the multivariate pattern path finishes
  without producing a factorisation, the handler now tries
  `try_bivariate_hensel_ir` before falling through to the unevaluated
  `Factor(...)` form.
- Added `cas_factor::{try_bivariate_hensel, BiPoly, Rat}` imports.

### Added — tests

`tests/hensel.rs` — 6 cases:

- `hensel_factor_x2_xy_minus_2y2_splits` — acceptance case
  `(x + 2y)(x - y)`.
- `hensel_factor_non_unit_leading_2x2_3xy_minus_2y2_splits` — leading
  coefficient ≠ 1.
- `hensel_factor_x3_minus_y3_splits` — multi-degree linear × quadratic.
- `hensel_factor_x2_plus_y2_plus_1_irreducible` — irreducible bivariate
  stays unevaluated.
- `hensel_factor_x2_minus_1_falls_through_to_univariate` — pure
  univariate regression: the existing path still produces
  `Mul(Add(1, x), Add(-1, x))`.
- `hensel_factor_x_plus_y_is_already_irreducible` — bare `x + y` stays
  unfactored.

Full suite: **200 passed** (194 prior + 6 net new).

## [0.14.0] — 2026-05-28

**Track B3 — Apart for repeated linear factors (Phase 48, Rust port).**

Lifts the multiplicity > 1 bail introduced in Track B1.
``Apart(P(x)/Q(x), x)`` now decomposes rational functions whose
denominator factors as ``∏_r (x − r)^{m_r}`` for *rational* ``r`` with
arbitrary multiplicity.  Each pole ``r`` of multiplicity ``m`` contributes
terms ``A_{r,1}/(x − r) + A_{r,2}/(x − r)² + … + A_{r,m}/(x − r)^m``
where the coefficients come from the Taylor expansion of
``φ(t) = P(r + t)/Q(r + t)`` around ``t = 0`` with
``Q(x) = den(x)/(x − r)^m``.  Then ``A_{r, m − j} = φ_j``.

This mirrors the Phase 48 algorithm added to Python ``symbolic-vm`` in
PR \#3927 and the TypeScript port at ``@coding-adventures/symbolic-vm``
0.14.0.  Acceptance: ``Apart(1/(k²(k+1)²), k)`` decomposes to
``2/(k+1) + 1/(k+1)² − 2/k + 1/k²`` (left-associated, roots ascending),
matching the Python reference byte-for-byte.

Denominators that still contain an irreducible quadratic factor on top of
the rational roots continue to bail to the unevaluated ``Apart(...)``
form — partial fractions over the rationals can't go further there.

### Added

- ``poly_taylor_expand_around_r`` — Taylor-expand a ``RatPoly`` around a
  rational point ``r`` to ``length`` coefficients using the binomial
  identity ``poly(r+t)_j = ∑_{i≥j} c_i · C(i, j) · r^(i−j)``.  Exact
  ``i128`` arithmetic throughout.
- ``poly_series_div`` — formal power-series division ``N(t)/D(t)`` to
  ``length`` terms via the recurrence
  ``Q_j = (N_j − ∑_{k≥1} D_k · Q_{j−k}) / D_0``.  Returns ``None`` when
  ``D(0) = 0`` (defensive guard against a repeated-root miscount).
- ``build_apart_term`` — IR builder for ``A / (x − r)^power`` with
  ``±1`` numerator elision matching ``apart_simple_roots``.
- ``binomial_i128`` — exact ``i128`` binomial helper used by the
  Taylor expansion.

### Changed

- ``apart_proper`` — Phase 48 generic path lifted in: when any
  multiplicity > 1, compute ``Q(x) = den(x)/(x − r)^m`` per root via
  successive ``rp_div``, Taylor-expand ``num`` and ``Q`` around ``r``,
  series-divide, and emit ascending-power terms via ``build_apart_term``.
  Phase 1 simple-roots fast path retained for the B1 regression tests
  (cheaper than Taylor + series division, preserves the existing IR
  shape).

### Removed

- The ``mult > 1 → None`` bail in ``apart_proper`` — the new code path
  handles the repeated-root case directly.

### Out of scope (deferred)

- Irreducible quadratic factors (``Apart`` over the rationals only).
- Algebraic-number roots beyond Q — would require an irrational-roots
  extension.

## [0.13.0] — 2026-05-28

**Track B1 — Apart simple-roots partial-fraction decomposition (Rust port).**

Ports the Phase 1 simple-root subset of Python's ``apart_handler`` from
``symbolic-vm/cas_handlers.py``.  ``Apart(P(x)/Q(x), x)`` now decomposes
rational functions whose denominator has only *distinct rational* roots
using the residue formula ``A_i = P(r_i) / Q'(r_i)``.  Improper fractions
(deg P ≥ deg Q) get a polynomial-division step first, then Apart on the
proper remainder.  Repeated roots (Phase 48 in the Python tree) and
denominators with irreducible quadratic factors leave the expression
wrapped in ``Apart(...)`` for downstream pipelines to handle.

This unblocks the deferred Rust port of the Phase 40 / 46 Apart-retry
telescope chain in ``cas-summation``.

### Added

- ``apart_handler`` registered under the ``"Apart"`` head in the symbolic
  backend's handler table.
- ``to_rational_ir`` IR → ``(num, den)`` bridge built on the existing
  ``RatPoly`` / ``RatC`` machinery (no new arithmetic substrate).
- ``rp_normalize`` / ``rp_evaluate`` / ``rp_rational_roots`` /
  ``rp_root_multiplicities`` / ``rp_power`` polynomial helpers and
  ``rp_to_ir_apart`` IR emitter, all sitting beside the existing
  rational-integration ``rp_*`` family.
- ``apart_simple_roots`` + ``apart_proper`` implementing the residue-
  formula decomposition.  ``apart_proper`` returns ``None`` (caller
  emits unevaluated ``Apart(...)``) when *any* multiplicity > 1 —
  Phase 48 is explicitly out of scope for this PR.
- ``tests/apart.rs`` with 6 acceptance cases mirroring the Track B1
  test plan in ``code/specs/macsyma-finish-plan.md``.

### Out of scope (deferred to follow-on tracks)

- Repeated linear factors (Phase 48 algorithm) — Track B3.
- Apart-retry telescope chain (Phase 40 + 46 composition) — Track B2.

## [0.12.0] — 2026-05-22

**Phase 47 — Nested-Add flattening (Rust port).**

Ports the Python ``symbolic-vm`` 0.71.0 Add-handler fix.  When either
binary ``Add`` operand is itself an ``Add(...)`` apply, the handler
now flattens the tree, sums numeric literals once, and rebuilds a
left-associated chain.  Example:

    Add(Add(k, 1), 1)  →  Add(k, 2)
    Add(Add(Add(k, 1), 1), 1)  →  Add(k, 3)

### Added

- **`flatten_add_leaves(node: &IRNode, out: &mut Vec<IRNode>)`** in
  ``src/handlers.rs`` — recursive walker that appends every
  non-``Add`` leaf of a nested ``Add`` tree to the ``out`` vector.

### Changed

- ``add_handler(simplify=true)`` now pre-checks whether either binary
  operand is an ``Add`` apply.  If so, it collects all non-``Add``
  leaves via ``flatten_add_leaves``, partitions into numerics vs
  symbolics, sums the numerics into a single ``Numeric`` via
  ``Add for Numeric``, and rebuilds a left-associated chain
  ``Add(...non_literals, lit_sum)`` — dropping the literal if it's
  zero, collapsing to a bare leaf if only one operand remains.
  Strict mode (``simplify=false``) keeps the original binary
  semantics.

### Added — tests

`tests/test_vm.rs` — 6 new ``phase47_*`` cases:

- ``phase47_nested_add_flattens``: ``Add(Add(k, 1), 1)`` → ``Add(k, 2)``.
- ``phase47_triply_nested_add``: 3-level flattening.
- ``phase47_add_constants_fold``: ``Add(Add(k, 2), 3)`` → ``Add(k, 5)``.
- ``phase47_constants_cancel_to_bare_symbol``: ``Add(Add(k, 1), -1)``
  → bare ``k``.
- ``phase47_non_nested_add_untouched``: ``Add(k, 1)`` — no rebuild.
- ``phase47_add_zero_still_simplifies``: regression for the existing
  ``x + 0 → x`` identity.

Full suite: **194 passed** (was 188; +6 net new).

## [0.11.0] — 2026-05-20

### Added — Phase 38: Weierstrass closed forms lifted to linear trig arguments

Mirrors Python `symbolic-vm` 0.63.0 (PR #3690) and TypeScript
`symbolic-vm` 0.11.0 (PR #3691).

The previous Phases 34–37 closed Weierstrass for
`∫ c / (a + b·trig(x)) dx` in all discriminant regimes (`a² > b²` arctan,
`a² = b²` degenerate, `a² < b²` log) but only when the trig argument
was the bare variable `x`.  Phase 38 generalises every branch to accept
any linear-in-`x` rational argument `α·x + β` (with `α, β ∈ ℚ`, `α ≠ 0`).

The mathematics is a single inner change of variable: with
`u = α·x + β` we have `du = α · dx`, so

    ∫ c / (a + b·sin(α·x + β)) dx
        = (1/α) · ∫ c / (a + b·sin u) du  (Phase 34/36/37 closed form in u)

The closed form is the existing one with `tan((α·x + β)/2)` substituted
for `tan(x/2)` and the outer constant scaled by `1/α`.  When `α = 1`
and `β = 0`, the new code path is bit-for-bit identical to the original
Phase 34–37 behaviour — full backwards compatibility.

### Added

- **`weierstrass_parse_linear_in_x(node, x) -> Option<(RatC, RatC)>`** in
  `handlers.rs` — parses a node into `(α, β)` rational pair when it
  represents `α·x + β`.  Handles bare `x`, scalar multiples, ADD/SUB
  with any operand ordering, and leading `Neg` wrappers.  Rejects
  nonlinear (`x²`) and pure-constant shapes by returning `None`.
  `α = 0` is filtered out so callers may rely on `α ≠ 0` throughout.
- **`weierstrass_build_linear_arg_ir(α, β, x)`** — builds the IR for
  `α·x + β`, collapsing trivial cases (`α=1, β=0 → x`, etc.) so the
  emitted `tan(arg/2)` carries the simplest equivalent argument.
- **`weierstrass_parse_const_times_trig_linear`** — supersedes the
  Phase 34 bare-`x` predecessor.  Returns `(c, head, α, β)` for any
  shape matching `c·sin(α·x + β)` or `c·cos(α·x + β)`.

### Changed

- **`weierstrass_parse_a_plus_b_sincos`** — now returns
  `(a, b, head, α, β)` instead of `(a, b, head)`.
- **`try_weierstrass_degenerate`, `try_weierstrass_log_form`,
  `try_weierstrass_one_over_linear_trig`** — accept an
  `arg_node: &IRNode` parameter representing `α·x + β` and substitute it
  into the `tan(arg/2)` construction.  The outer `c ← c/α` scaling is
  applied once at the dispatcher entry, so each branch's closed form is
  otherwise unchanged.

### Added — tests

`tests/test_vm.rs` — 9 new Phase 38 cases (1 promoted from the prior
fallthrough deferral):

- `phase38_sin_two_x_closes` — `∫ 1/(2 + sin 2x) dx`.
- `phase38_cos_three_x` — `α = 3` cos variant.
- `phase38_sin_x_plus_constant_phase` — pure phase shift `α = 1, β = 1`.
- `phase38_sin_two_x_plus_phase` — full `α = 2, β = 1`.
- `phase38_rational_alpha` — `α = 1/2`.
- `phase38_negative_alpha` — `α = −2` sign-flipped.
- `phase38_degenerate_branch_under_substitution` — `(1 + cos 2x)`
  exercises the Phase 35 degenerate path with α=2.
- `phase38_log_form_under_substitution` — `(1 + 2·sin 2x)` exercises
  the Phase 36 log path with α=2.
- `phase38_fallthrough_nonlinear_argument` — `sin(x²)` is correctly
  left unevaluated.
- `phase38_fallthrough_symbolic_alpha` — `sin(α·x)` with symbolic
  α defers gracefully.

The pre-existing `phase34_fallthrough_non_bare_argument` is removed
(commented-out with a pointer to the new Phase 38 success test).

All 188 tests pass (179 prior + 9 net new).

### Still deferred

- Symbolic coefficients (`a`, `b`, `α`, or `β` non-numeric) — needs an
  assumption context to decide discriminant sign.
- Trig argument involving `x²` or other nonlinear forms — out of scope
  for Weierstrass.

## [0.10.0] — 2026-05-20

### Changed — Phase 37: Weierstrass log form cos branch covers `b < −|a|`

Mirrors Python `symbolic-vm` 0.62.0 (PR #3683) and TypeScript
`symbolic-vm` 0.10.0 (PR #3685).

`try_weierstrass_log_form` cos branch: removed the
`b_minus_abs_a` and `b_minus_a` positivity guards.  The same log
formula with `Abs` wrapping handles both `b > |a|` and `b < −|a|`.

Tests: 3 new in `tests/test_vm.rs` (one promoted from
fallthrough):
- `phase37_cos_negative_b_now_closes` — `∫ 1/(1 − 2·cos x) dx`
- `phase37_cos_negative_b_with_negative_a` — `∫ 1/(−1 − 3·cos x) dx`
- `phase37_cos_negative_b_with_numerator_coefficient` — `∫ 5/(1 − 2·cos x) dx`

Full suite: **179 passed** (177 prior + 3 new − 1 promoted).

## [0.9.0] — 2026-05-20

### Added — Phase 36: Weierstrass log form for `a² < b²`

Mirrors Python `symbolic-vm` 0.61.0 (PR #3672) and TypeScript `symbolic-vm`
0.9.0 (PR #3674).  Closes the deferred `a² < b²` branch of Phase 34 by
emitting the explicit log-form closed solution:

    ∫ c/(a + b·sin x) dx = (c/D)·log|(a·tan(x/2)+b−D)/(a·tan(x/2)+b+D)| + C
    ∫ c/(a + b·cos x) dx = (c/D)·log|(D+(b−a)·tan(x/2))/(D−(b−a)·tan(x/2))| + C

where `D = √(b²−a²) > 0`.  Sin handles any nonzero rational `a`; cos
requires `b > |a|` strictly.

New free function `try_weierstrass_log_form(c, a, b, trig_head, x)` in
`src/handlers.rs` replaces the prior `return None` in the `disc < 0`
arm of `try_weierstrass_one_over_linear_trig`.  The log argument is
wrapped via `apply_node("Abs", ...)` so the closed form evaluates
numerically across the integrand's singularities.

Tests: 5 new `#[test]` functions in `tests/test_vm.rs` plus one
promoted from fallthrough (`phase36_a_less_than_b_sin_now_closes`,
replacing `phase34_fallthrough_a_less_than_b`).

Full suite: **177 passed** (172 prior + 5 net new).

## [0.8.0] — 2026-05-18

### Added — Phase 35: degenerate `a² = b²` Weierstrass cases

Closes the four degenerate branches that Phase 34 (0.7.0) deliberately
deferred:

    ∫ 1/(a + a·sin x) dx = -2 / (a · (tan(x/2) + 1))
    ∫ 1/(a − a·sin x) dx =  2 / (a · (1 − tan(x/2)))
    ∫ 1/(a + a·cos x) dx =  tan(x/2) / a
    ∫ 1/(a − a·cos x) dx = -1 / (a · tan(x/2))     (= -cot(x/2)/a)

Each formula is exact — no `Sqrt`, no `Atan` — because the
post-substitution quadratic in `u = tan(x/2)` factors as `a(u ± 1)²`
for sin and reduces to `2a` (constant) or `2a·u²` for cos.

#### Added (`src/handlers.rs`)

- **`try_weierstrass_degenerate(c, a, b, trig_head, x)`** — Phase 35
  helper.  Pattern-matches the four `(b == a, b == -a) × (SIN, COS)`
  combinations and emits the corresponding closed form.  Returns
  `None` for the pathological `a == 0` sub-case (zero denominator).

- Updated `try_weierstrass_one_over_linear_trig` (Phase 34) to call
  `try_weierstrass_degenerate` when `disc == 0` and to return `None`
  (defer) when `disc < 0` (log form, still open).

#### Tests (`tests/test_vm.rs`)

6 new `#[test]` functions — same scenarios as the Python and TS Phase
35 suites:

- `phase35_a_equals_b_now_closes` (replaces the prior
  `phase34_fallthrough_a_equals_b` deferment test).
- `phase35_one_minus_sin_closes` — sin, b = −a.
- `phase35_one_plus_cos_closes` — cos, b = a → tan(x/2).
- `phase35_one_minus_cos_closes` — cos, b = −a → −cot(x/2).
- `phase35_with_numerator_coefficient` — c=5 scaling.
- `phase35_rational_coefficients` — a=b=3/2.

Each verifies the closed form via the existing
`phase34_numerical_derivative` helper at sample points avoiding the
`tan(x/2)` pole at `x = π` and the `1/(1−cos x)` pole at `x = 0`.

Full suite: 124 passed (118 prior + 6 net new).

## [0.7.0] — 2026-05-18

### Added — Phase 34: Weierstrass substitution for ∫ 1/(a + b·sin/cos x) dx

Ports Python `symbolic-vm` 0.59.0 Phase 34 to Rust.  The substitution
`u = tan(x/2)` reduces `∫ 1/(a + b·sin x) dx` and `∫ 1/(a + b·cos x) dx`
to rational functions of `u` whose closed form is an arctan whenever
`a² > b²`.  Closed forms:

    ∫ 1/(a + b·sin x) dx  =  (2/√(a²−b²)) · arctan((a·tan(x/2) + b)/√(a²−b²))
    ∫ 1/(a + b·cos x) dx  =  (2/√(a²−b²)) · arctan(√((a−b)/(a+b)) · tan(x/2))

For exact-rational `a, b` with `a² > b²` (and `a > 0` for the cos
branch) the integrator now closes the form directly.  A numerator
constant `c` simply scales the result.

#### Deferred to a later phase

- `a² < b²` — log form on `(a·tan(x/2)+b ± √(b²−a²))` (sign analysis).
- `a² = b²` — degenerate, reduces to a rational in `tan(x/2)`.
- `a ≤ 0` for the cos branch — `(a−b)/(a+b)` sign analysis.
- Symbolic `a` or `b` — discriminant sign undecidable without an
  assumption context (the Rust port has no assumption system).
- Non-bare trig arguments (e.g. `sin(2x)`).

#### Added (`src/handlers.rs`)

- **`try_weierstrass_one_over_linear_trig(num, den, x)`** — Phase 34
  entry point.  Matches `c / (a + b·sin/cos(x))` shapes with rational
  c, a, b.  Returns the closed-form `IRNode` or `None`.
- **`weierstrass_parse_a_plus_b_sincos(node, x)`** — structural matcher
  returning `(a, b, trig_head_str)` for ADD/SUB with both operand
  orderings.
- **`weierstrass_parse_const_times_trig_x(node, x)`** — matches
  `c·sin(x)`, `c·cos(x)`, `sin(x)`, `cos(x)`, and `Neg`-wrappings.
- **`weierstrass_sqrt_fraction_ir(rc)`** — emits `Sqrt(p/q)` IR, folding
  to a clean rational when both numerator and denominator are perfect
  integer squares (reuses the existing `i128_sqrt` helper).
- **`node_to_rc(node)`** — `IRNode` → `RatC` converter for Integer and
  Rational literals.

Wired into the `(DIV, [c, denom])` arm of `integrate` after the existing
`1/x` case.  The Weierstrass path requires `!depends_on(c, x)` so it
never fires for `x/(2+sin x)` style integrands.

#### Tests (`tests/test_vm.rs`)

14 new `#[test]` functions mirroring the Python and TS Phase 34 suites:

- Closed-form structure: `∫ 1/(2 + sin x) dx` contains an `Atan` node.
- Numeric-derivative verification at multiple sample points for both
  sin and cos.
- Perfect-square discriminant folds `Sqrt` away (a=5, b=3 → disc=16).
- Numerator coefficient scales the closed form (`∫ 3/(2 + sin x) dx`).
- Rational coefficients (a=3/2, b=1/2; disc=2).
- Operand-order robustness (`∫ 1/(sin x + 2) dx` still closes).
- Four fallthrough guarantees: a²<b², a²=b², non-bare arg, symbolic `a`.
- Regression: `∫ sin(x) dx = −cos(x)` unchanged; `∫ 1/cos(x) dx` is
  NOT misinterpreted as a Weierstrass case.

New helpers in the test file: `phase34_subst`, `phase34_eval_at`,
`phase34_numerical_derivative`, `is_unevaluated_integrate`.  The
existing `eval_at` / `contains_head` helpers are kept intact (the
Phase 34 numerical evaluator routes through the full `SymbolicBackend`
to evaluate `Tan` / `Sqrt` / `Atan` correctly, while the existing
`eval_at` is a manually-coded subset for Phases 26-28).

## [0.6.0] — 2026-05-18

### Added — Phases 29–33: algebraic simplification rules

Extends the symbolic backend with five new rule families that fire on every
re-evaluation of the affected expressions.  All rules are guarded by the
`simplify` flag so the `StrictBackend` (numeric-only) is unaffected.

#### Phase 29 — Abs and Sqrt algebraic rules

New free functions:
- `frac_gcd`, `frac_make`, `frac_mod`, `frac_from_ir` — fraction arithmetic
  helpers used internally by the Phase 33 π-multiple detection and by the
  Phase 29–32 handlers.

**`abs_handler`** (new — `Abs` head previously fell through unhandled):
- Numeric fold: `Abs(-3) → 3`, `Abs(-p/q) → p/q`.
- Idempotency: `Abs(Abs(x)) → Abs(x)`.
- Negation strip: `Abs(Neg(x)) → Abs(x)`.
- Mul-neg strip: `Abs(Mul(-1, x)) → Abs(x)`.
- Even-power identity: `Abs(Pow(x, 2k)) → Pow(x, 2k)` for even integer 2k ≥ 2.
- Registered as `"Abs"` in `build_handler_table`.

**`sqrt_handler`** (replaces `single_trig` factory):
- Perfect-square detection: `sqrt(4) → 2`, `sqrt(9) → 3`, etc.
- Even-exponent rewrite `sqrt(x^{2k})`:
  - k even → `Pow(x, k)` (e.g. `sqrt(x^4) = x^2`)
  - k odd → `Abs(x^k)` (e.g. `sqrt(x^2) = |x|`, `sqrt(x^6) = |x^3|`)

#### Phase 30 — Log / Exp cancellation rules

**`log_handler`** (replaces `single_trig`):
- `log(exp(x)) → x`  (structural cancellation).
- Special value `log(1) → 0`; non-positive inputs left unevaluated.

**`exp_handler`** (replaces `single_trig`):
- `exp(log(x)) → x`.
- `exp(n·log(x)) → x^n`  (both `Mul(n, log(x))` and `Mul(log(x), n)`).
- Special value `exp(0) → 1`.

**Regression note**: `D(x^x, x)` now returns `Mul(Pow(x,x), Add(log(x), x/x))`
because `exp(x·log(x))` eagerly reduces to `x^x`.  Test updated.

#### Phase 31 — Trig / hyperbolic negation symmetry and arc-cancellation

**Odd** (`sin_handler`, `tan_handler`, `sinh_handler`, `tanh_handler`):
- `f(Neg(x)) → Neg(f(x))` with `vm.eval` recursive descent.

**Even** (`cos_handler`, `cosh_handler`):
- `f(Neg(x)) → f(x)` (Neg stripped, recurse).

**Arc-cancellation** in `sin`/`cos`/`tan`/`sinh`/`cosh`/`tanh`:
- `sin(Asin(x)) → x`, `cos(Acos(x)) → x`, `tan(Atan(x)) → x`
- `sinh(Asinh(x)) → x`, `cosh(Acosh(x)) → x`, `tanh(Atanh(x)) → x`

#### Phase 32 — Inverse trig / hyperbolic odd symmetry

**Odd** (`atan_handler`, `asin_handler`, `asinh_handler`, `atanh_handler`):
- `f(Neg(x)) → Neg(f(x))`.

**`acos_handler`** — reflection:
- `acos(Neg(x)) → Sub(Symbol("%pi"), acos(x))`.

**`acosh_handler`** — keeps `single_trig` factory (domain `[1, ∞)`, no symmetry).

#### Phase 33 — Trig exact values at rational multiples of π

New free functions:
- `try_pi_multiple(arg: &IRNode) -> Option<Frac>` — detects float ≈ q·π and
  structural patterns `%pi`, `Neg(%pi)`, `Mul(n, %pi)`, `Div(%pi, n)`,
  `Div(Mul(n, %pi), d)`.
- `p33_sqrt_over(n, d) -> IRNode` — helper building `Div(Sqrt(n), d)`.
- `p33_neg(v: IRNode) -> IRNode` — wraps `Neg`.
- `sin_pi_table(p, q) -> Option<IRNode>` — 16-entry exact sin table (period 2).
- `cos_pi_table(p, q) -> Option<IRNode>` — 16-entry exact cos table (period 2).
- `tan_pi_table(p, q) -> Option<IRNode>` — 7-entry exact tan table (period 1).

`sin_handler`, `cos_handler`, `tan_handler` each call `try_pi_multiple` on the
argument and look up the table before the numeric fold.

`tan(π/2)` (undefined) is left unevaluated.

**Tests added** (48 new tests across all 5 phases):
- Phase 29: 8 tests (abs/sqrt rules)
- Phase 30: 4 tests (log/exp cancellation + power form)
- Phase 31: 12 tests (trig+hyperbolic symmetry and arc-cancellation)
- Phase 32: 5 tests (inverse trig odd symmetry + acos reflection)
- Phase 33: 19 tests (sin/cos/tan π-multiples including negative q and regression)

Helper added to test file: `fn eval(expr: IRNode) -> IRNode` — thin wrapper
around `symbolic().eval(expr)` used by the new Phase 29–33 tests.

## [0.5.0] — 2026-05-18

### Added — Phase 28: general IBP for poly×log(Q) and poly×atan(Q)

Extends symbolic integration to handle products of a polynomial `P(x)` with
`log(Q(x))` or `atan(Q(x))` where `Q(x)` is a **non-linear** polynomial with
rational coefficients.  Uses the IBP formula:

  ∫ P·log(Q) dx  =  R·log(Q) − ∫ R·Q′/Q dx
  ∫ P·atan(Q) dx =  R·atan(Q) − ∫ R·Q′/(1+Q²) dx

where R = ∫P (polynomial antiderivative, constant = 0).

**New functions:**

- `try_log_poly_product(transcendental, poly, x)` — Phase 28 log IBP handler;
  skips linear Q (deferred to Phase 3) and integrates the residual via
  `integrate_rational_simple_rp`.
- `try_atan_poly_product(transcendental, poly, x)` — Phase 28 atan IBP handler;
  skips linear Q and integrates the residual via `integrate_rational_simple_rp`.
- `integrate_rational_simple_rp(num_rp, denom_rp, denom_ir, x)` — targeted
  rational function integrator for Phase 28 residuals.  After polynomial long
  division:
  - **Case A**: remainder = c·D′ → c·log(D)
  - **Case B**: constant remainder / quadratic ax²+b with rational √(b/a)
                → r₀/(a₂·√(a₀/a₂))·atan(x/√(a₀/a₂))
- `close_remainder_over_d(r, d, d_prime, d_ir, x)` — attempts Cases A/B for
  the post-division remainder polynomial.
- `eval_numeric_node(node)` — evaluates a closed IR numeric expression
  (handling Mul/Div/Neg/Add/Sub of exact rationals) to a `RatC` value;
  used by `rp_from_poly_vec` to extract rational coefficients from compound
  coefficient nodes produced by `to_polynomial_coeffs`.
- `is_linear_in(expr, x)` — returns true iff the expression is a non-constant
  linear polynomial in `x`; used to guard the Phase 28 arms.

**Rational polynomial arithmetic helpers** (used internally by Phase 28):
`gcd128`, `rc`, `rc_neg`, `rc_add`, `rc_sub`, `rc_mul`, `rc_div`, `rc_to_ir`,
`eval_numeric_node`, `rp_from_poly_vec`, `rp_deg`, `rp_is_zero`, `rp_coeff`,
`rp_add`, `rp_sub_poly`, `rp_mul_scalar`, `rp_shift`, `rp_mul`, `rp_deriv`,
`rp_integrate`, `rp_div`, `rp_to_ir`, `rp_proportional`, `i128_sqrt`,
`rc_sqrt`.

The arithmetic layer uses `RatC = (i128, i128)` and `RatPoly = Vec<RatC>` with
`i128` to give headroom for cross-multiplications without overflow.

**Dispatch wiring:**
- MUL branch: after Phase 27, tries `try_log_poly_product(a,b,x)` and
  `try_atan_poly_product(a,b,x)` (and symmetric variants) for both-depend cases.
- Bare function path: `∫ log(Q) dx` (P=1) and `∫ atan(Q) dx` (P=1) are
  detected via new `(LOG, [q]) if …!is_linear_in` and `(ATAN, [q]) if …` arms.

**Examples that now evaluate:**
- `∫ log(x²+1) dx` = x·log(x²+1) − 2x + 2·atan(x)
- `∫ x·log(x²+1) dx` = (x²/2)·log(x²+1) − x²/2 + ½·log(x²+1)
- `∫ x²·log(x²+1) dx` = (x³/3)·log(x²+1) − 2x³/9 + 2x/3 − (2/3)·atan(x)
- `∫ x·atan(x²) dx` = (x²/2)·atan(x²) − ¼·log(1+x⁴)

**Fallthrough cases** (correctly left unevaluated):
- `∫ atan(x²) dx` — residual 2x²/(1+x⁴) requires irrational partial fractions
- `∫ atan(x) dx` — linear Q, not intercepted by Phase 28

**Tests added** (9 new tests):
- `phase28_log_x2p1_is_closed` — closed-form structure check
- `phase28_log_x2p1_numeric` — numerical correctness ∫₀¹ log(x²+1) dx
- `phase28_x_log_x2p1_is_closed` — closed-form structure check
- `phase28_x_log_x2p1_numeric` — numerical correctness
- `phase28_x2_log_x2p1_numeric` — numerical correctness
- `phase28_atan_x2_fallthrough` — stays unevaluated
- `phase28_x_atan_x2_is_closed` — closed-form structure check
- `phase28_x_atan_x2_numeric` — numerical correctness
- `phase28_regression_log_x_still_phase3` — Phase 3 regression
- `phase28_regression_atan_x_stays_unevaluated` — linear atan regression

## [0.4.0] — 2026-05-16

### Added — Phase 26: log-power integration via IBP reduction

- `is_log_of_x(node, x)` — guard helper: returns `true` when `node` is
  `Log(x)` for bare integration variable `x`.
- `to_polynomial_coeffs(expr, x)` — extracts polynomial coefficients from an
  IR expression; returns `Vec<(degree, coeff_node)>` or `None`.
  Handles constants, `x`, `x^k`, `c·f`, `f·c`, ADD, SUB, NEG.
- `poly_log_power_term(k, n, x)` — closed form of `∫ x^k · log(x)^n dx` for
  k ≥ 0, n ≥ 1, via the IBP reduction formula:
  `G_{k,m}(x) = x^(k+1)/(k+1) · log(x)^m  −  m/(k+1) · G_{k,m-1}(x)`.
- `try_log_power_product(transcendental, poly, x)` — handles
  `∫ Q(x) · log(x)^n dx` for integer n ≥ 2 by term-by-term application
  of `poly_log_power_term`.
- Standalone `∫ log(x)^n dx` (n ≥ 2) via new `(POW, [base, exp])` match arm.

### Added — Phase 27: trig-of-log integration via u = log(x) substitution

- `trig_log_integral(trig_head, k, x)` — closed form of `∫ x^k · trig(log(x)) dx`:
  - `∫ xᵏ sin(log x) dx = x^(k+1)·((k+1)sin(log x)−cos(log x))/((k+1)²+1)`
  - `∫ xᵏ cos(log x) dx = x^(k+1)·((k+1)cos(log x)+sin(log x))/((k+1)²+1)`
- `try_trig_log_product(transcendental, poly, x)` — handles
  `∫ Q(x)·sin(log(x)) dx` and `∫ Q(x)·cos(log(x)) dx`.
- Standalone `∫ sin(log(x)) dx` and `∫ cos(log(x)) dx` via new
  `(SIN|COS, [inner]) if is_log_of_x(inner, x)` match arms.

## [0.3.0] — 2026-05-14

### Added

- Added EllipticE (second kind) integration recognition:
  - `∫₀^(π/2) √(1-k²sin²θ) dθ` → `EllipticE(k)` (complete)
  - `∫ √(1-k²sin²θ) dθ` → `EllipticE(θ, k)` (incomplete)
- Added EllipticPi (third kind) complete integration recognition:
  - `∫₀^(π/2) 1/((1+n·sin²θ)·√(1-k²sin²θ)) dθ` → `EllipticPi(n, k)`
- New helper functions: `elliptic_second_kind_radicand`, `complete_elliptic_second_kind`,
  `incomplete_elliptic_second_kind`, `extract_characteristic_n`,
  `elliptic_third_kind_params`, `complete_elliptic_third_kind`

## [0.2.0] — 2026-05-14

### Added

- `Integrate` recognises canonical elliptic first-kind forms, returning
  `EllipticF(theta, k)` for the incomplete integral and `EllipticK(k)` for the
  complete `[0, %pi/2]` definite integral.
- `SymbolicBackend` now installs canonical `Factor` handling backed by
  `cas-factor`, including common-symbolic-factor extraction for additive
  multivariate expressions before univariate integer factorization.
- `Factor` extracts the greatest common integer content (GCD of all term
  coefficients) and intersection of common symbolic powers before attempting
  specific pattern matches. For example `factor(2*x + 4*y)` → `2*(x + 2*y)`,
  `factor(2*x*y + 2*x*z)` → `2*x*(y + z)`, and `factor(2*x^2*y - 2*y)` →
  `2*y*(x+1)*(x-1)` (the univariate residual is factored recursively).
- `Factor` recognises bivariate perfect-square trinomials such as
  `x^2 + 2*x*y + y^2` and rewrites them as `(x + y)^2`.
- `Factor` recognises bivariate difference-of-squares expressions such as
  `x^2 - y^2` and rewrites them as `(x - y) * (x + y)`.
- `Factor` recognises bivariate cubic identities such as `x^3 - y^3` and
  `x^3 + y^3`, rewriting them to their canonical linear/quadratic products.
- `Factor` recognises four-term bilinear grouping such as
  `x*y + x*z + y + z` and rewrites it as `(x + 1) * (y + z)`.
- `Factor` extracts shared multivariate integer content such as
  `2*x*y + 2*x*z`, including all-negative shared signs.
- `Factor` recognises four-term bivariate perfect-cube expansions such as
  `x^3 + 3*x^2*y + 3*x*y^2 + y^3` and `x^3 - 3*x^2*y + 3*x*y^2 - y^3`,
  rewriting them as `(x + y)^3` and `(x - y)^3` respectively.
- `SymbolicBackend` installs a `D` derivative handler for symbolic-only
  differentiation of arithmetic, elementary, hyperbolic, and inverse
  hyperbolic expressions; `StrictBackend` continues to reject `D` as an
  unknown head.
- Numeric and symbolic handlers for reciprocal hyperbolic heads `Coth`,
  `Sech`, and `Csch`, including `sech(0) = 1` and undefined-at-zero checks for
  `coth`/`csch`.

## [0.1.1] — 2026-04-28

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `symbolic-vm` package.
- `Backend` trait with `lookup`, `bind`, `on_unresolved`, `on_unknown_head`,
  `handler_for`, `rules`, `hold_heads`.
- `Handler` type alias: `Arc<dyn Fn(&mut VM, IRApply) -> IRNode + Send + Sync>`.
- `VM` struct with `eval(IRNode) -> IRNode` and `eval_program(Vec<IRNode>) -> Option<IRNode>`.
- `BaseBackend` — shared environment + held-heads for the two reference backends.
- `StrictBackend` — numeric-only evaluator; panics on unbound symbols or unknown heads.
- `SymbolicBackend` — Mathematica-style; unbound names stay as free variables;
  algebraic identities (`x+0→x`, `x*1→x`, `0*x→0`, `x^0→1`, etc.) are applied.
- Full handler table (34 handlers): `Add`, `Sub`, `Mul`, `Div`, `Pow`, `Neg`, `Inv`,
  `Sin`, `Cos`, `Tan`, `Exp`, `Log`, `Sqrt`, `Atan`, `Asin`, `Acos`, `Sinh`, `Cosh`,
  `Tanh`, `Asinh`, `Acosh`, `Atanh`, `Equal`, `NotEqual`, `Less`, `Greater`,
  `LessEqual`, `GreaterEqual`, `And`, `Or`, `Not`, `If`, `Assign`, `Define`, `List`.
- Exact rational arithmetic: `Numeric` enum preserving `Int(i64)`, `Rat(i64, i64)`,
  `Float(f64)` intermediate values; checked overflow falls back to `Float`.
- User-defined function support via `Define(name, List(params), body)` records,
  evaluated by substitution.
- 52 integration tests + 2 doc-tests; all passing.

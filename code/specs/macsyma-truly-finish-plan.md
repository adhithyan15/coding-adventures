# MACSYMA Truly-Finish Plan

> **🎉 Closed** — 2026-05-29 with an omnibus PR landing Tracks F–N.  All
> eight tracks shipped across Python, TypeScript, and Rust; every
> CHANGELOG `Unreleased` section in the MACSYMA pipeline is empty.
> MACSYMA is considered feature-complete for mainstream Maxima 5.x
> parity.  No items deferred.
>
> | Track | Description | Landed |
> |---|---|---|
> | F | TS + Rust `macsyma-runtime` 0.4.0 — assumption-aware abs/sqrt/log | #5354 |
> | G1 | Python symbolic-coefficient Weierstrass + `AssumptionContext` compound-relation store | #5361 |
> | G2 | TS + Rust port of G1 | #5363 |
> | H1 | Python Gosper's algorithm for hypergeometric summation | #5366 |
> | H2 | TS + Rust port of H1 | #5369 |
> | I1 | Python closed-form transcendental infinite sums (zeta/eta/Taylor) | #5382 |
> | I2 | TS + Rust port of I1 | #5387 |
> | J1 | Python series-expansion limit fallback (Taylor-after-L'Hôpital) | #5574 |
> | J2 | TS + Rust port of J1 | #5583 |
> | K1 | Python n-variate Hensel factoring + symbolic-vm bridge | #5590 |
> | K2 | TS + Rust port of K1 | #5599 |
> | L1 | Python Lie point-symmetry first-order ODE | omnibus |
> | L2 | TS + Rust port of L1 | omnibus |
> | M1 | Python MACSYMA `load("name")` directive + `orthopoly` package | omnibus |
> | M2 | TS + Rust port of M1 | omnibus |
> | N | Closure (this banner + spec sweep + cas-simplify Unreleased flush) | omnibus |
>
> Spec history below is preserved as the planning context.  New work is
> feature-driven; no further phases against this plan.

> **Status**: Planning document, drafted 2026-05-29 after a deep audit of
> the previous `macsyma-finish-plan.md` (closed 2026-05-28). That plan
> shipped its 11 sub-tracks but explicitly carved out four large items
> as non-goals and left several mainstream-Maxima parity gaps that a
> follow-up audit surfaced. This document enumerates everything still
> pending for **full Maxima 5.x mainstream-feature parity** and orders
> it into atomic tracks.
>
> **Exit criterion**: when every sub-track in the table below has merged
> and every CHANGELOG "Unreleased" section in the MACSYMA pipeline
> packages is empty, this document gets a 🎉 closure banner and MACSYMA
> is considered feature-complete.  No items deferred.

> **Read first**: `lessons.md` — in particular the "STOP and generalise"
> entry. Tracks that look like a per-shape grid must be re-shaped to a
> single algorithm.

---

## Goal

Match the mainstream feature surface of Maxima 5.x (the open-source
descendant of MACSYMA) across Python, TypeScript, and Rust. Concretely
the following capability gaps remain:

1. **Assumption-aware direct evaluation** of `abs`, `sqrt`, `log` in
   TypeScript and Rust (Python has it; TS+Rust have the wiring but no
   release).
2. **Symbolic-coefficient Weierstrass** — `∫ 1/(a + b·sin x) dx` with
   `a, b` symbolic, when an assumption context decides the discriminant
   sign.
3. **Hypergeometric / Gosper closed-form summation** — for term ratios
   `a(k+1)/a(k)` that are rational in `k`.
4. **Closed-form transcendental infinite sums** — recognise canonical
   series for `∑ 1/k²`, `∑ (-1)^(k-1)/k`, `∑ 1/k!`, `∑ x^k/k!`, etc.
5. **Series-expansion limit fallback** — for limits where direct
   substitution and L'Hôpital don't close (e.g. `lim_{x→0} (sin x - x)/x³`).
6. **n-variate Hensel factoring** — extend the bivariate Hensel to
   three or more variables.
7. **Lie point-symmetry first-order ODE reduction** — bounded slice
   covering scaling and translation symmetries.
8. **MACSYMA `:load` package system** — runtime directive to extend the
   handler table on demand.

The previous finish-plan's "non-goals" entries 6 / 7 / 8 (limit machine,
n-variate factor, ODE solvers beyond Frobenius) are addressed here as
bounded engineering slices, not unbounded research.  Tracks J, K, L
deliver concrete acceptance criteria with finite test sets.

---

## Tracks

Eight tracks, each Python-first then TypeScript+Rust port, plus a
closing spec-update track.  Each PR < 800 lines and < 12 test cases
unless explicitly otherwise.

### Track F — Assumption-aware direct evaluation (release)

| # | Description | Languages | Acceptance |
|---|---|---|---|
| F | Cut the dated TS + Rust `macsyma-runtime` release that ships the existing Unreleased `abs/sqrt/log` assumption wiring | TS + Rust | `assume(x >= 0); sqrt(x^2)` returns `x` (not `abs(x)`) in TS + Rust REPL.  CHANGELOG entry dated, package.json / Cargo.toml bumped to 0.4.0. |

### Track G — Symbolic-coefficient Weierstrass

| # | Description | Languages | Acceptance |
|---|---|---|---|
| G1 | Lift the `a, b ∈ ℚ` restriction in `_try_weierstrass_*` (Python `symbolic-vm`) | Python | `assume(a^2 > b^2); integrate(1/(a + b*sin(x)), x)` returns the arctan form with symbolic `a, b`.  Undecidable assumptions fall through unevaluated. |
| G2 | Port to TS + Rust using the F-track assumption context | TS + Rust | Same closed form on 4 standard cases. |

### Track H — Hypergeometric / Gosper summation

| # | Description | Languages | Acceptance |
|---|---|---|---|
| H1 | Gosper's algorithm in Python `cas-summation`: detect rational `a(k+1)/a(k)`, solve for antidifference | Python | `∑_{k=1}^N k·2^k = 2 + (N-1)·2^(N+1)`, `∑_{k=0}^N binomial(N,k) = 2^N`, plus 2 more. |
| H2 | Port to TS + Rust | TS + Rust | Same outputs on the H1 cases. |

### Track I — Closed-form transcendental infinite sums

| # | Description | Languages | Acceptance |
|---|---|---|---|
| I1 | Pattern-match canonical infinite series to closed forms in Python `cas-summation` | Python | `∑_{k=1}^∞ 1/k^2 → %pi^2/6`, `∑_{k=1}^∞ (-1)^(k-1)/k → log(2)`, `∑_{k=0}^∞ 1/k! → %e`, `∑_{k=0}^∞ x^k/k! → exp(x)` (as a symbolic-in-x identity). |
| I2 | Port to TS + Rust | TS + Rust | Same outputs. |

### Track J — Series-expansion limit machine

| # | Description | Languages | Acceptance |
|---|---|---|---|
| J1 | Add a Taylor-series fallback in Python `cas-limit-series` after direct + L'Hôpital fail | Python | `lim_{x→0} (sin x - x)/x^3 = -1/6`, `lim_{x→0} (1 - cos x)/x^2 = 1/2`, `lim_{x→0} (exp(x) - 1 - x)/x^2 = 1/2`. |
| J2 | Port to TS + Rust | TS + Rust | Same outputs. |

### Track K — n-variate Hensel lifting

| # | Description | Languages | Acceptance |
|---|---|---|---|
| K1 | Extend Python `cas-factor`/`hensel.py` to n ≥ 3 variables via iterated bivariate lifting | Python | `factor(x^2 + x*y + x*z - 2*y^2 - 3*y*z - z^2)` returns a two-factor decomposition with rational coefficients. |
| K2 | Port to TS + Rust | TS + Rust | Same outputs. |

### Track L — Lie point-symmetry first-order ODE

| # | Description | Languages | Acceptance |
|---|---|---|---|
| L1 | In Python `cas-ode`, detect scaling symmetry `(x,y) → (λx, λᵏy)` and translation symmetry for first-order ODEs that fall through the existing families. Reduce to quadrature via the symmetry. | Python | `ode2(y' = (y^2 + x*y)/x^2, y, x)` (scaling-invariant, k=1) closes to a Bernoulli-equivalent reduction. |
| L2 | Port to TS + Rust | TS + Rust | Same outputs. |

### Track M — MACSYMA `:load` package system

| # | Description | Languages | Acceptance |
|---|---|---|---|
| M1 | Add `:load("pkg")` directive in Python `macsyma-runtime` that registers extra handlers on demand. Refactor existing orthogonal-polynomial heads (`LegendreP`, `ChebyshevT`, etc.) into a loadable `orthopoly` package. | Python | `:load("orthopoly"); legendre_p(3, x)` returns the 3rd Legendre polynomial; without the load, the symbol is unevaluated. |
| M2 | Port to TS + Rust | TS + Rust | Same behaviour. |

### Track N — Closure

| # | Description |
|---|---|
| N | Add closure banner to this spec, update `macsyma-completion.md` and `spice-macsyma-pending-work.md`.  Final PR.  Sanity-check that no CHANGELOG in the MACSYMA pipeline has a non-empty Unreleased section. |

---

## Execution order

```
PR 1: This spec doc                  →  baseline
PR 2: Track F (TS + Rust release)    →  easy win, clears Unreleased
PR 3: Track G1 (Python Weierstrass)  →  Python first
PR 4: Track G2 (TS + Rust)
PR 5: Track H1 (Python Gosper)
PR 6: Track H2 (TS + Rust)
PR 7: Track I1 (Python infinite sums)
PR 8: Track I2 (TS + Rust)
PR 9: Track J1 (Python series limit)
PR 10: Track J2 (TS + Rust)
PR 11: Track K1 (Python n-variate Hensel)
PR 12: Track K2 (TS + Rust)
PR 13: Track L1 (Python Lie symmetry)
PR 14: Track L2 (TS + Rust)
PR 15: Track M1 (Python :load)
PR 16: Track M2 (TS + Rust)
PR 17: Track N (closure)
```

Each PR follows the existing alternation discipline (Python gap → TS+Rust
port → next Python gap → next TS+Rust port → …) with implementation +
tests + version bump + CHANGELOG entry + Agent-driven security review.

Each PR is babysat with a 3-minute recurring timer until CI is green and
merge is conflict-free.  When a PR merges, the loop picks up the next item.

---

## Anti-patterns to refuse

Inherited from `macsyma-finish-plan.md` (still applicable):

1. **Helper-per-count.** No `_two_sqrt_*`, `_three_log_*`, … One generic
   that counts.
2. **Version bumps too fine.** Real semver bumps `0.X.0`, not `0.0.X`.
3. **CHANGELOG copy-paste.** Identical-modulo-integers entries mean the
   helper is identical-modulo-integers.
4. **Test-per-count.** Cover edge cases, not just `N=1, 2, 3`.

And one new one for this plan:

5. **Algorithm-per-equation-shape.** Gosper is *one* algorithm.
   Lie-symmetry detection is *one* algorithm.  n-variate Hensel is *one*
   algorithm.  Per-shape branches inside the algorithm are fine; per-shape
   helpers at the top level are not.

---

## Spec updates required after each track

- `code/specs/spice-macsyma-pending-work.md` — strike completed items,
  link merged PRs.  The 🎉 closure banner from 2026-05-28 stays as a
  historical marker of the previous plan; this spec's closure banner
  joins it.
- `code/specs/macsyma-completion.md` — move completed tracks to
  Complete.

When all eight tracks land, this spec gets its own 🎉 closure banner.

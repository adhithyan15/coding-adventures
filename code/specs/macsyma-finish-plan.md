# MACSYMA Completion — Finish Plan

> **🎉 Closed** — 2026-05-28 with the merge of the Track E2 PR
> (generic IBP fallback ported to TypeScript and Rust).  All ten
> sub-tracks across the five tracks landed:
>
> | Track | Sub-track | PR |
> |---|---|---|
> | A1 | Phase 86 generic recogniser port (TS + Rust) | #4552 |
> | A2 | Delete 42 redundant grid helpers (all 3 langs) | #4557 |
> | B1 | Apart simple-roots port (TS + Rust) | #4558 |
> | B2 | Apart-retry telescope chain port (TS + Rust) | #4559 |
> | B3 | Apart repeated-linear-factors port (TS + Rust) | #4560 |
> | C1 | Frobenius / power-series ODE (Python) | #4561 |
> | C2 | Frobenius port (TS + Rust) | #4562 |
> | D1 | Bivariate Hensel lifting (Python) | #4563 |
> | D2 | Bivariate Hensel port (TS + Rust) | #4564 |
> | E1 | Generic IBP fallback (Python) | #4569 |
> | E2 | Generic IBP port (TS + Rust) | this PR |
>
> Spec history below is preserved as the planning context.  New work
> is feature-driven; no further phases against this plan.

> **Status**: Planning document, drafted 2026-05-28 after the
> cas-summation overboard incident (74 PRs closed + cleanup PR
> #4545). Sets the boundary of "what does done look like" for the
> MACSYMA pipeline and orders the remaining work into atomic phases.
>
> **Read first**: `lessons.md` — especially the entry on
> "STOP and generalise when you find yourself writing the Nth
> variant". The MACSYMA finish work has several patterns that look
> like a (N, M)-grid; the spec calls those out so we don't repeat
> the cas-summation mistake.

## Goal

A MACSYMA pipeline (`macsyma-lexer` → `macsyma-parser` → `macsyma-compiler`
→ `symbolic-ir` → `symbolic-vm` → `macsyma-runtime`) that handles the
realistic CAS workload of a graduate engineering / mathematics user:

- Algebra: factor, expand, solve (univariate + linear systems), simplify.
- Calculus: limits, derivatives, integration (Risch ~ 90% complete plus
  elliptic integrals), Taylor series.
- ODEs: first-order families + 2nd-order const-coeff + Euler-Cauchy +
  variation of parameters + named ODEs (Legendre / Bessel / Hermite /
  Chebyshev) **+ Frobenius power series**.
- Linear algebra: full matrix operation set (already complete).
- Discrete math: number theory (already complete), summation.
- Cross-language: Python is the reference, TypeScript and Rust ports
  must stay within one minor version.

When this spec's exit criteria are met, MACSYMA is considered done
for the purposes of this repo. New work is feature-driven (e.g.
Maple frontend) rather than gap-driven.

## Non-goals

- Full symbolic limit machine (a separate large project).
- General multivariate factoring beyond Hensel lifting (Newton
  polytope methods, etc.).
- Symbolic ODE solvers beyond Frobenius (e.g. Lie symmetry, Risch–Bronstein
  for elementary ODEs).
- Performance tuning. Correctness first; runtime is "fast enough for
  REPL use".

## Exit criteria

The pipeline is "done" when:

1. All cas-summation hand-written N-Sqrt × M-Log × polynomial helpers
   (Phases 59–85) are deleted; only the generic (Phase 86) remains.
   Phase 86 ports to TypeScript and Rust have landed.
2. `Apart` (partial-fraction decomposition) is ported to TypeScript
   and Rust, unblocking the deferred Phase 40/46/48 ports of the
   Apart-retry telescope chain.
3. `ode2` recognises Frobenius / power-series ODEs in all three
   languages.
4. `Factor` handles generic bivariate polynomials with no common
   factor via Hensel lifting (e.g. `factor(x²+xy-2y²)`) in all three
   languages.
5. General IBP fallback exists for integration when neither factor
   integrates alone, in all three languages.
6. All three `cas-summation` packages are at the same major.minor
   version and pass `npm test` / `cargo test` / `pytest`.
7. `code/specs/spice-macsyma-pending-work.md` is updated to mark
   each phase ✅ with the merged PR number.

Estimated total: **15 PRs**, alternating Python-gap and TS/Rust-port,
with each PR < 500 lines and < 6 test cases unless explicitly
otherwise.

---

## Tracks

The remaining work splits into five independent tracks. Each track
is one or more PRs, and tracks can execute in parallel.

### Track A — Generic-cleanup follow-through (lowest risk first)

| # | Description | Languages | Acceptance | Status |
|---|---|---|---|---|
| A1 | Phase 86 generic recogniser port | TS + Rust | `npm test` / `cargo test` green; matches Python behavior on the 6 new Phase 86 test cases. | ✅ #4552 |
| A2 | Delete the 42 redundant N-Sqrt × M-Log × polynomial helpers (Phases 59–85) | All 3 | Sweep: pre/post test counts equal; no behavior change. | ✅ #4557 |

**Lesson check**: A1 and A2 are intentionally separated. Bundling
delete-with-port is tempting but risks port hiding a regression. Land
A1 first, observe sweep numbers, then A2.

### Track B — Apart port (highest leverage)

Porting `Apart` to TypeScript and Rust unlocks the deferred Phase 40
(Apart-retry telescope), Phase 46 (Apart-retry constant numerator
widening), and Phase 48 (Apart for repeated linear factors) ports.

| # | Description | Languages | Acceptance | Status |
|---|---|---|---|---|
| B1 | `Apart` simple-roots (Phase 1 algorithm) port | TS + Rust | `Apart(1/(x²-1), x)` returns `1/(2(x-1)) - 1/(2(x+1))` in both. | ✅ #4558 |
| B2 | Apart-retry telescope chain port (Phase 40 + Phase 46) | TS + Rust | `∑_{k=1}^∞ 1/(k(k+1)) = 1` closes via Apart + telescope. | ✅ #4559 |
| B3 | Apart for repeated linear factors (Phase 48 algorithm) port | TS + Rust | `Apart(1/(k²(k+1)²), k)` decomposes correctly. | ✅ #4560 |

**Lesson check**: B1, B2, B3 are sequential dependencies — do not parallel
them. Each PR ports one algorithm; tests in the target language
validate against the Python reference output.

### Track C — Frobenius ODE

Power-series / Frobenius method for variable-coefficient 2nd-order
linear ODEs around regular singular points.

| # | Description | Languages | Acceptance | Status |
|---|---|---|---|---|
| C1 | Frobenius algorithm in Python `cas-ode` | Python | `ode2(x²y'' + xy' + (x²-1/4)y = 0, y, x)` → series solution recognised as `BesselJ(1/2, x)` family. | ✅ #4561 |
| C2 | Frobenius port | TS + Rust | Output matches Python on 3 standard ODE test cases (Bessel, Legendre, hypergeometric). | ✅ #4562 |

**Lesson check**: Frobenius is a real algorithm, not a grid. One helper
per language, not one per equation type. Named ODE recognition
(Bessel, Legendre, etc.) is already in Phase 21; Frobenius is the
fallback for un-named regular-singular-point ODEs.

### Track D — Generic multivariate Hensel lifting

| # | Description | Languages | Acceptance | Status |
|---|---|---|---|---|
| D1 | Bivariate Hensel lifting in Python `cas-factor` | Python | `factor(x²+xy-2y²) → (x+2y)(x-y)` and similar. | ✅ #4563 |
| D2 | Hensel lifting port | TS + Rust | Same outputs as Python on 5 standard cases. | ✅ #4564 |

**Lesson check**: Hensel lifting is one algorithm; do not write one helper
per polynomial shape. Pattern shapes can route through the algorithm
but should not bypass it.

### Track E — General IBP integration fallback

| # | Description | Languages | Acceptance | Status |
|---|---|---|---|---|
| E1 | Generic IBP table-driven fallback in Python `symbolic-vm` integration handler | Python | `∫ x·sin(x²) dx` and similar reach a closed form via IBP recursion. | ✅ #4569 |
| E2 | Generic IBP fallback port | TS + Rust | Same outputs on 5 standard cases. | ✅ this PR |

---

## Anti-patterns to refuse

Drawn from the cas-summation overboard incident (#4467–#4544 + 27 already
merged). Any PR matching one of these patterns must be closed during
review, not merged:

1. **Helper-per-count.** Functions whose names embed a fixed integer
   (`_two_sqrt_*`, `_three_log_*`, `_n_log_m_sqrt_*`). If two helpers
   differ only by a count, write one generic helper that counts.
2. **Version bumps too fine.** Bumping by 0.001–0.01 per PR for a
   single phase is a smell. Real semver phases bump by `0.X.0`. The
   cas-summation grid bumped by `0.006` per PR and reached `v2.373.0`
   — that's the same "Nth variant" smell expressed as version churn.
3. **CHANGELOG copy-paste.** When a new CHANGELOG section is
   character-by-character identical to the previous section modulo two
   integers, the helper is also identical modulo two integers. Refactor
   the helper; don't ship the CHANGELOG.
4. **Test-per-count.** Tests that just instantiate the same template
   with `N=1`, `N=2`, `N=3`, … without exercising the helper's edge
   cases (`N=0`, unrecognised factor, negative-leading polynomial).

A reviewer noticing two PRs in a row matching these patterns should
ask for a generic before merging the second.

---

## Execution order

```
Track A1 → Track A2  (must be sequential — see lesson check)
Track B1 → Track B2 → Track B3  (sequential — algorithmic stack)
Track C1 → Track C2  (sequential — Python first, then port)
Track D1 → Track D2  (sequential — Python first, then port)
Track E1 → Track E2  (sequential — Python first, then port)
```

The five tracks are independent of each other. Start with **A1**
(smallest, lowest risk), then **B1** (highest leverage). C, D, E in
any order after that.

Each PR follows the existing alternation discipline:

- Python gap PR → TS+Rust port PR → next Python gap → next TS+Rust port → …
- Each PR includes implementation + tests + version bump + CHANGELOG
  entry + Agent-driven security review.
- Each PR is babysat with a 3-minute recurring timer until CI is
  green and merge is conflict-free.
- When a PR merges, the loop picks up the next pending item.

---

## Spec updates required after each track

The following document must be updated each time a track completes:

- `code/specs/spice-macsyma-pending-work.md` — strike completed
  items, link merged PRs.
- `code/specs/macsyma-completion.md` — move completed tracks from
  "In Progress" to "Complete".

This spec (`macsyma-finish-plan.md`) becomes a closed document when
all five tracks land.

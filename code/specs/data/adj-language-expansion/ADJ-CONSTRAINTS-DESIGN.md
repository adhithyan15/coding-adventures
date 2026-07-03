# ADJ-CONSTRAINTS — dimensional types, currency/dates/times, inequalities, symbols, and constraint solving

> Design of record for growing adj-lang from a **computation** language (extract values → compute with
> a derivation tree) into a **constraint** language: the model extracts typed values *and* the
> policy's constraints; the engine **solves** them deterministically — solve-for-x, feasibility /
> contradiction, optimization — with **provenance flowing from every solved value back to the
> constraints and their source bytes**. Strict dimensional typing throughout.
>
> This is the sequel to [DESIGN.md](DESIGN.md) (steps 1–3b: predicates, typed values, `let`+arithmetic,
> the derivation tree — all merged) and [STEP3-let-arithmetic-PLAN.md](STEP3-let-arithmetic-PLAN.md).

## 1. Why — adjudication *is* constraint solving

Eligibility (`income ≤ threshold ∧ age ≥ 18`), proration (`split bonus by tenure`), deadlines
(`claim within 365 days of purchase`), break-even (`solve for the premium that zeroes the margin`),
allocation (`maximise coverage subject to a cap`) — these are linear equations, inequalities, boolean
rule-trees, and small optimizations. If the **model** solves them, the answer is un-auditable and
wrong-by-arithmetic (the E2/HLE failure mode). The fix is the standing thesis
([[feedback_deterministic_is_probabilistic_special_case]], [[project_cpu_bound_reasoning_problog]]):
**the model decomposes** messy text into typed values + the constraint structure; the **CPU engine
solves**, and every solved value / infeasibility certificate is traced back to the constraints that
forced it. One engine: a solved constraint feeds the **differential** exactly as a predicate does
(feasible ⇒ one verdict, infeasible ⇒ another), so deterministic = the saturating limit of
probabilistic still holds.

User-confirmed scope (all four classes, all four outputs, strict units):
- **Solve:** linear eq+ineq · nonlinear/algebraic · boolean/SAT · optimization (LP).
- **Produce:** solved values · feasibility/contradiction certificate · feed-a-verdict · optimal value.
- **Units:** strict dimensional (reject `usd + eur`, `dollars + days` unless an explicit conversion
  fact is present).

## 2. The dimensional type system (the new foundation — strict)

A value is a `(magnitude, dimension)` pair. The magnitude reuses exact arithmetic
(`numeric-tower::Rational` = BigRational, falling back to f64 only at the numeric-solver boundary).
The **dimension** is the new piece:

| surface           | term shape                              | dimension            |
|-------------------|-----------------------------------------|----------------------|
| `18000`           | `Num`                                   | `Scalar`             |
| `money(18000,usd)`| `Compound{money,[Num,usd]}`             | `Money("usd")`       |
| `quantity(40,mg_dl)` | `Compound{quantity,[Num,mg_dl]}`     | `Unit("mg_dl")`      |
| `percentage(40)`  | `Compound{percentage,[Num]}`            | `Percent`            |
| `date(2025,1,15)` | `Compound{date,[Num,Num,Num]}`          | `Date`               |
| `time(14,30,0)`   | `Compound{time,[Num,Num,Num]}`          | `TimeOfDay`          |
| `datetime(…)`     | `Compound{datetime,[date,time]}`        | `DateTime`           |
| `duration(365,days)` | `Compound{duration,[Num,days]}`      | `Duration("days")`   |
| `true`/`false`    | `Atom`                                  | `Boolean`            |
| `?x` (a symbol)   | `Compound{sym,[Atom]}`                  | `Unknown(sort)`      |

**Dimensional algebra** (a small, total set of rules — the engine, not the model, enforces them):
- `add/sub`: operands must share a dimension (`usd+usd`, `days+days`); `Money(a)+Money(b)` with `a≠b`
  is an error unless a conversion fact exists (§2.1). `Date + Duration → Date`; `Date − Date →
  Duration`. Scalar is the identity dimension.
- `mul/div`: `Money / Money → Scalar` (a ratio), `Money × Scalar → Money`, `Unit(a)/Unit(a) → Scalar`
  (units cancel — the CSF:serum ratio is dimensionless), `Percent` applied as `× p/100`. Cross-unit
  products carry a composite dimension tag (`Unit("usd·day")`) the faithfulness gate can inspect.
- comparisons (`< ≤ > ≥ = ≠`): only between same-dimension values; the result is `Boolean`.

This is the `numeric_magnitude` reader of step 2 generalised: instead of "read the leading number," the
engine reads `(magnitude, dimension)` and the **dimension travels through the derivation tree** so the
faithfulness gate (step 3d) can reject `usd + days`.

### 2.1 Conversions (explicit facts only)
`convert money(1, usd) = money(0.92, eur)` (a fact with provenance) licenses `usd↔eur`; the engine
applies the rate as a normal `Op` node so the conversion is itself in the derivation tree.
`days_between`/`date_add` (datetime-core) are the `Date×Duration` operators. No implicit coercions.

## 3. Symbols & constraints — the new surface

```
% the model extracts unknowns + the policy's constraints (NOT their solution)
symbol premium : money(usd)
symbol months  : scalar

observe base_rate = money(1200, usd)
observe claim     = money(5000, usd)

% constraints the engine must satisfy
constrain premium = base_rate + claim * percentage(10)        % a linear equation
constrain premium <= money(2000, usd)                          % an inequality
constrain months >= 6

solve for { premium, months }                                  % → solved values + provenance
% or:  minimize premium subject to { … }                       % → optimal value + assignment
% or:  check { … }                                             % → SAT / UNSAT(core) feasibility
```

- `symbol <name> : <sort>` declares an unknown with a dimensional sort.
- `constrain <expr> <relop> <expr>` asserts an (in)equality; `relop ∈ { = ≠ < ≤ > ≥ }`. Reuses the
  step-3 `expr` grammar, extended so operands may be symbols.
- `solve for { … }` / `minimize|maximize <expr> subject to { … }` / `check { … }` are the three
  drivers (the three user-chosen outputs; "feed-a-verdict" is `check` wired into the differential).
- Inequalities are now **first-class** both as constraints and (already) as predicate gates.

## 4. Solver dispatch — classify, then route (mostly reuse)

A `solve`/`check`/`minimize` block builds a **typed constraint system**, which a dispatcher classifies
and routes. Each value/cert is provenance-tagged.

| class detected | backend | reuse / build |
|---|---|---|
| linear **equations**, exact | `cas-solve::solve_linear_system` (Gaussian over ℚ) | **reuse** |
| linear **integer** eq+ineq feasibility | `constraint-engine::LiaTactic` (Cooper) | **reuse** |
| boolean / rule-trees | `constraint-engine::SatTactic` (DPLL) | **reuse** |
| linear **real** eq+ineq feasibility (QF_LRA) | — (engine returns `Unknown` today) | **BUILT (C1): Fourier–Motzkin over ℚ** in `adj-constraint-solver::check` (self-contained checked i128 rational; overflow → `Unknown`) |
| linear **optimization** | — | **BUILT (C2): Fourier–Motzkin projection** in `adj-constraint-solver::optimize` (objective-bounded `z`, project out decision vars; *not* simplex) |
| nonlinear univariate (deg ≤ 4) | `cas-solve::{solve_quadratic,cubic,quartic}` | **reuse** |
| nonlinear numeric / higher degree | `cas-solve::nsolve_poly` (Durand–Kerner), `cas-mnewton` | **reuse / extend** |
| algebraic roots ℚ[√d] | `cas-algebraic` | **reference** |
| mixed theories | `constraint-engine` Nelson-Oppen dispatch | **reuse** |

The crux gaps are **QF_LRA + simplex** (adjudication is real-valued: money, ratios, percentages) and
the **dimensional layer + surface sublanguage + provenance bridge**. Everything else is wiring.

## 5. Provenance & output (the whole point)

- **Solved value** → a derivation-tree-like `Solution { symbol, value, from_constraints: [ConstraintId] }`;
  each `ConstraintId` cites the source bytes of the constraint (same Provenance/CV machinery as clauses).
  Reuses the step-3a `DerivationNode` shape extended with a `FromSolve` node.
- **Infeasibility** → an **UNSAT core / IIS** (irreducible infeasible subset): the *minimal* set of
  conflicting constraints, each cited. This is the machine-checked "these two rules contradict" that
  catches golden-rulebook bugs ([[project_mycin_prototype]]).
- **Optimal value** → the objective value + the achieving assignment + the binding constraints (the
  ones tight at the optimum), all cited.
- **Feed-a-verdict** ✅ (E2) → the constraint result feeds the differential: feasible ⇒ verdict A,
  infeasible ⇒ verdict B. One engine; no new verdict logic. **Implemented without a grammar change**:
  `adj-lang-cli` runs the constraint engine first, maps each outcome to a STATUS atom (`feasible` /
  `infeasible` / `solved` / `optimal` / `unbounded`; `Unknown`/`Unsupported` → nothing), injects it as
  an observed fact into the KB *before* `decide`, and an existing
  `contributes <lr> from <status> to <verdict>` clause fires through the ordinary contribution + proof
  machinery.
- **FromSolve / proof descent** ✅ (E3) → the verdict's proof descends into the solver certificate.
  **Divergence from the original plan:** implemented *without* a new `logic-engine`
  `DerivationOrigin::FromSolve` — it lives entirely in the `adj-lang-cli` renderer. A contribution step
  whose evidence is a constraint STATUS atom gets a `"solver": …` field carrying that constraint's full
  result (the IIS `core`, the assignment, the optimum) via the existing `*_json` renderers. So a
  verdict's proof step reads `…,"solver":{"outcome":"unsat","core":[0,1,2]}` — the verdict *and* the
  exact conflicting constraints, in one auditable tree, no engine change.

## 6. Reuse map (grounded — verified by source read)

| need | crate | path | verdict |
|---|---|---|---|
| exact rational arithmetic | `numeric-tower` (BigRational) | `code/packages/rust/numeric-tower/` | reuse-as-is |
| linear systems (Gaussian/ℚ) + poly roots ≤4 + Durand–Kerner + inequality | `cas-solve` | `code/packages/rust/cas-solve/` | reuse-as-is |
| SAT (DPLL) + LIA (Cooper) + Nelson-Oppen | `constraint-engine` (+ `constraint-core` predicate AST) | `code/packages/rust/constraint-engine/` | reuse; **build LRA + simplex tactics** |
| Newton (nonlinear numeric) | `cas-mnewton` | `code/packages/rust/cas-mnewton/` | extend |
| symbolic matrices (row-reduce/rank/nullspace/LU) | `cas-matrix` | `code/packages/rust/cas-matrix/` | reference/extend |
| dates/durations (Howard-Hinnant, pure Rust) | `datetime-core` | `code/packages/rust/datetime-core/` | reuse-as-is |
| expression IR + evaluator | `symbolic-ir` / `symbolic-vm` | `code/packages/rust/symbolic-{ir,vm}/` | reuse for nonlinear bridge |
| derivation tree + differential + provenance | `logic-engine` (`compute`, `differential`, `proof_dag`, `provenance`) | `code/packages/rust/logic-engine/` | reuse/extend |
| **dimensional types / units / currency** | — | — | **MUST BUILD** |
| **LP / simplex (optimization)** | — | — | **MUST BUILD** |

## 7. Roadmap (small specs-first PRs, each green + security-reviewed + babysat)

**Track A — dimensional types & richer values (independent, lands first):**
- **A1** Dimensional core in logic-engine: `Dimension` enum + `Dimensioned { magnitude, dim }`;
  generalise `numeric_magnitude` → `dimensioned_value`; dimensional add/sub/mul/div rules + errors.
- **A2** Currency + conversions: `money`, `convert … = …` facts, same/cross-currency algebra.
- **A3** Dates/times/durations via `datetime-core`: `days_between`, `date_add`, `before/after`,
  `date/time/datetime/duration` typed values; deadline predicates (`elapsed <= 365`).
- **A4** Faithfulness gate over dimensions (step-3d, dimensional half): reject unit-mismatched ops.

**Track B — symbols & constraint sublanguage (depends on A1):**
- **B1** Surface: `symbol`, `constrain`, `check`/`solve for`/`minimize|maximize` grammar + AST + adapter;
  a typed `ConstraintSystem` IR. No solver yet — just build + classify + dump.
- **B2** Wire the **reuse** backends: linear equations → `cas-solve`; boolean → `SatTactic`; integer →
  `LiaTactic`. `Solution` / UNSAT-core provenance + `FromSolve` proof origin + CLI render.

**Track C — the build gaps (depends on B1):**
- **C1** ✅ QF_LRA feasibility (Fourier–Motzkin over ℚ) — real-valued feasibility/contradiction.
  **Divergence from the original plan:** built in `adj-constraint-solver::check` (the dispatcher that
  already owns `check` semantics and C3's nonlinear bridge), over a self-contained **checked i128
  rational** (`Rat`, overflow → `Unknown` — never a silent wrap) — *not* as a new `constraint-engine`
  tactic. `check` now layers two procedures:
  the exact linear-**integer** tactic (`LiaTactic`, B2c) runs first; when it punts (`Unknown`) or when a
  constraint is non-integer, **or when it reports integer-`Unsat` (which over ℝ may still be feasible —
  e.g. `2x = 1`)**, the Fourier–Motzkin layer decides **real** feasibility and returns a rational
  witness (`SatReal`). A constraint set is `Unsat` only when *both* the integer and real layers reject
  it. `!=` (disjunctive, non-convex) stays `Unknown`. Caps on intermediate-inequality count and
  coefficient magnitude bound the worst-case blow-up.
- **C2** ✅ `minimize|maximize` optimization + binding constraints. **Divergence:** implemented by
  **Fourier–Motzkin projection** (bound a fresh `z` by the objective, project out the decision
  variables, read `z`'s least upper bound as the optimum) reusing the C1 machinery — *not* a separate
  simplex tableau. Returns `Optimal{value,assignments,binding}` / `Unbounded` / `Infeasible{core}` /
  `Unknown` (the last covers an open supremum from a strict bound). Exact over the checked `Rat`.
- **C3** Nonlinear bridge: `cas-solve`/`cas-mnewton` for `constrain` with quadratic+ terms.

**Track D — payoff:**
- **D1** Worked adjudication examples (eligibility, proration, deadline, break-even, allocation) as
  `.adj` + golden tests, end-to-end through `adj-lang-cli` at **0 answer-time model calls**.
- **D2** Fold into the Haiku run (DESIGN step 8): Haiku extracts values+constraints, engine solves.

Ordering: A1→A2/A3 and B1→B2 can run in parallel after A1; C1 is the highest-value build (real-valued
constraints) and unblocks C2; nonlinear (C3) and optimization (C2) are the long tail.

## 8. Invariants & non-goals

**Invariants** (carry from DESIGN §7):
- The model emits **no solved values** — only extracted typed values + constraint structure.
- Every solved value / infeasibility cert is **reconstructable from the solver certificate** without
  the model, and cites the constraints (→ source bytes) that forced it.
- **Strict dimensions**: no implicit unit/currency coercion; conversions are explicit, provenanced facts.
- One engine: a solved/checked constraint feeds the differential; verdict families from the differential.
- Exact arithmetic (ℚ) wherever the solver is exact; float only at the numeric-root boundary, screened
  for non-finite (as `compute` already does).

**Non-goals (this round):** quantified/first-order constraints; nonlinear *optimization* (only linear
LP); floating-point interval arithmetic; a general SMT portfolio beyond Bool/LIA/LRA; physical-unit
libraries beyond what adjudication needs (currency, time, and opaque `Unit(tag)` cancellation).

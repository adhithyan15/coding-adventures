# Changelog

All notable changes to this project will be documented in this file.

## [0.18.0] - 2026-06-16 — precedence priority is a named ENUM, not an integer (ADJ73 PR-A)

### Changed (breaking — nothing released; per user decision 1)

- **`Rule::priority` is now `Priority`** (a named enum) instead of `i64`. Tiers, totally
  ordered lowest→highest: `Default < Specific < Authoritative < Mandatory`. `Default` is the
  implicit tier (existing rules unchanged). `Rule::with_priority` now takes a `Priority`.
- **`GovernedAnswer::priority` is now `Standing`** (new enum): `Standing::Rule(Priority)` or
  `Standing::Asserted` (a ground fact — outranks every rule tier, replacing the old `i64::MAX`
  sentinel). `Standing` derives `Ord` (Asserted greatest), so the resolver compares tiers
  without magic numbers.
- The two `adjudication-connector` `Rule{}` literal sites set `priority: Priority::Default`.

### Rationale

Raw integers were magic-numbery; named tiers read correctly in grounded rulebooks and are the
simplest *grounded precedence principle* ("a higher tier wins"). Richer, byte-provenanced
precedence (a grounded `context-precedence` rulebook with lex-superior / recency / appeal-status
meta-rules) is ADJ73 PR-B; the recursive grounded design is now spec'd in
`code/specs/ADJ73-defeasible-rule-precedence.md` §2.3 + §7.

### Unchanged

- Resolution semantics, opt-in-per-predicate `declare_functional`, and back-compat of
  `enumerate_all` are exactly as in 0.17.0. All 101 + 5 (govern) + 4 (precedence integration)
  tests pass.

## [0.17.0] - 2026-06-16 — defeasible rule precedence (ADJ73 PR-1)

### Added

- **`Rule::priority: i64`** (default `0`, builder `Rule::with_priority(p)`) — a rule's
  precedence among *conflicting* derivations. Higher defeats lower.
- **`KnowledgeBase::declare_functional(functor, arity)`** — mark a predicate FUNCTIONAL on
  its last argument (at most one value per key = the preceding args). Two derivations that
  share the key but differ on the last argument *conflict*.
- **`govern::enumerate_governing(query, kb) -> GovernedResult`** — runs `enumerate_all`, then
  resolves conflicting answers by precedence as a post-pass: the unique maximum-priority answer
  in a conflict group **governs**; the rest are **`Defeated { by }`**; a tie at the maximum is
  surfaced as **`ConflictPeer`** (never silently resolved). A fact-derived answer has priority
  `i64::MAX` (asserted truth outranks any rule). `GovernedResult::governing()` /
  `has_conflict()` helpers.

### Unchanged (back-compat)

- `enumerate_all` and SLD search are **untouched**: a query over predicates none of which are
  declared functional returns every answer as `Governing` (today's semantics exactly).
  Precedence is opt-in per predicate. The new `Rule.priority` field defaults to `0`; the two
  `adjudication-connector` `Rule{}` literal sites set it explicitly.

### Scope

- PR-1 ships the **functional-predicate conflict relation + total integer priority**. Explicit
  `conflict {}` sets and the `context_order` partial order (ADJ73 §2, the legal-context
  precedence) are PR-1b — they reuse this same resolution post-pass. Surface syntax in adj-lang
  is PR-2. See `code/specs/ADJ73-defeasible-rule-precedence.md`.

## [0.16.0] - 2026-06-14 — `KnowledgeBase::fact(id)` accessor (MYCIN-2026 REL-3)

### Added

- **`KnowledgeBase::fact(&self, id: FactId) -> Option<&Fact>`** — resolve a
  proof's `via_facts` (or a `DerivationOrigin::FromFact`) back to the firing fact,
  in particular its `provenance`, so a relational recall binding query's answer
  can be returned WITH the citing edge's source.

## [0.15.0] - 2026-06-14 — mandatory `Fact::provenance` for relational edges (MYCIN-2026 REL-2)

### Added / Changed (breaking)

- **`Fact::provenance: Provenance`** (mandatory — every fact is accountable) +
  the `Fact::with_provenance(p)` builder. A ground relational edge (adj-lang's
  `relate` clause) lowers to a `Fact` that carries its citation, so a binding
  query's answer (`? deficient_in(tay_sachs, $E)` → `hexosaminidase_a`) is
  returned WITH a proof — the byte-provenanced source that justifies the edge.
  Ordinary `observe`d facts carry `Provenance::unattributed()` — the explicit
  "no source" value, not a silent `None`. **Breaking:** the field is `Provenance`,
  not `Option<Provenance>`; the two `Fact` builders default it to
  `Provenance::unattributed()`, so all existing construction sites compile
  unchanged, but any code matching `fact.provenance` as an `Option` must adapt.
  `add_fact` preserves it.

## [0.14.0] - 2026-06-11 — dimensional faithfulness gate (ADJ constraints track A4)

### Changed

- **`compute` is now dimension-aware.** Alongside the f64 magnitude, the
  evaluator tracks each value's `Dimension` (read from its fact via
  `dimensioned_value`) and checks every binary op through `Dimension::combine`,
  so a unit-mismatched formula — `usd + days`, `usd + eur` without a conversion
  — is a clean **`ComputeError::DimensionMismatch`** instead of a
  silently-wrong number. This is the faithfulness gate: the engine, not the
  model, decides a category error is a category error.
  - `usd + usd → usd`; `money / money → scalar` (a dimensionless ratio,
    e.g. debt-to-income); `money × scalar → money`; bare-number formulas stay
    `Scalar` (the pre-A4 numeric behaviour is unchanged).
- **`Derived` gains a `dim: Dimension`** field — the inferred dimension of the
  computed value, so a predicate firing over it (`csf_ratio <= 0.4`) knows
  `csf_ratio` is a `Scalar` and the audit shows the unit. (Additive: callers
  that pass a `Derived` through unchanged are unaffected.)

### Added

- `KnowledgeBase::observed_dimensioned(slot)` — the dimensioned (`magnitude +
  Dimension`) observation of a slot with its `FactId`, for the gate.
- `ComputeError::DimensionMismatch { op, lhs, rhs }` carrying the two clashing
  unit tags for the audit reader.

## [0.13.0] - 2026-06-11 — date arithmetic (deadlines & durations, ADJ constraints track A3)

### Added

- **`datetime` module** — calendar arithmetic on the CPU for adjudication
  deadlines ("is the claim within 365 days of purchase?"). A date is a *point
  in time*, so it gets the new `Dimension::Date` and its arithmetic lives here,
  not in the generic `Dimension::combine` (which now rejects any `Date`
  operand, steering callers to these functions):
  - `days_between(a, b)` → a `Duration("days")` dimensioned value (so a deadline
    predicate `elapsed <= 365` fires over it).
  - `date_add(date, days)` → the resulting `(y, m, d)` (`Date + Duration → Date`).
  - `before(a, b)` / `after(a, b)` → a boolean ordering.
  - `read_date` validates month (`1..=12`) and day (`1..=days_in_month`, leap-aware)
    so `date(2025, 13, 40)` is a clean `None`; `read_duration_days` reads
    `duration(n, days|weeks)`.
- `days_from_civil` / `civil_from_days` — Howard Hinnant's public-domain
  proleptic-Gregorian ↔ day-ordinal algorithm, **inlined** (not a dependency).
  The repo's `datetime-core` is the right library but pulls `numeric-tower` /
  `r-vector` / `wall-clock` — too heavy for the core engine; the algorithm is
  ~25 lines of exact integer math, so we inline it and keep `logic-engine`
  dependency-free.
- `Dimension::Date`; `dimensioned_value` now returns `None` for `date`/`time`/
  `datetime` terms (their leading field is a year, not a scalar magnitude).

### Security (from /security-review, both LOW, fixed in-PR)

- All ordinal arithmetic is overflow-safe on attacker-controlled fields: `read_date`
  bounds the year to `±1_000_000`; `date_add` uses `checked_add` + an ordinal
  bound; `read_duration_days` uses `checked_mul` + a bound; and the raw
  `days_from_civil`/`civil_from_days` helpers are now `pub(crate)` (internal),
  so the public surface (`days_between`/`date_add`/`before`/`after`) can't be
  handed an unbounded `i64` that would overflow.

Time-of-day and full datetime arithmetic are a follow-up; this slice is dates +
durations (the deadline case). See
`code/specs/data/adj-language-expansion/ADJ-CONSTRAINTS-DESIGN.md`.

## [0.12.0] - 2026-06-11 — currency conversions (ADJ constraints track A2)

### Added

- **`conversion` module** — the only thing that licenses a cross-currency
  operation (which A1 made a `DimError::Mismatch`): an **explicit, provenanced
  conversion fact**. `Conversion::new("usd", "eur", 0.92)` = "1 usd = 0.92 eur",
  carrying a `Provenance` citation. `ConversionTable::rate(from, to)` resolves a
  rate via a direct fact, its inverse (`1/rate`), or the identity; no transitive
  chaining (a missing path is a clean `None`, never a guess).
- `convert_value(value, target, table)` converts a `Dimensioned` between
  currencies/units (money→money, unit→unit), re-tagging the dimension;
  `Scalar`/`Percent` are not convertible.
- `add_or_sub(subtract, lhs, rhs, table)` — dimension-aware add/sub that resolves
  a currency mismatch by converting `rhs` into `lhs`'s dimension via the rate
  (so `100 usd + 92 eur` = `200 usd` given `1 usd = 0.92 eur`), and still rejects
  genuinely incompatible kinds (`usd + days`). `ConvError::{NoRate, NotConvertible}`.
- `Conversion::try_new` validates the rate and returns `ConvError::BadRate`
  for a non-finite/non-positive value (the entry point a surface-`convert`
  lowerer should call, mirroring the LR/probability guards); `new` is the
  panicking trusted/test convenience. `convert_value`/`add_or_sub` screen for a
  non-finite result (`ConvError::NonFinite`), matching `ComputeError::NonFinite`
  so a converted value can't silently flow non-finite into a verdict.

Engine-only; the surface `convert money(1,usd) = money(0.92,eur)` statement and
recording the rate as a derivation-tree `Op` land with the constraint
sublanguage (B1) and the dimensional faithfulness gate (A4). See
`code/specs/data/adj-language-expansion/ADJ-CONSTRAINTS-DESIGN.md`.

## [0.11.0] - 2026-06-11 — dimensional types (strict units, ADJ constraints track A1)

### Added

- **`dimension` module** — every value gets a `Dimension`
  (`Scalar`/`Money(ccy)`/`Unit(tag)`/`Percent`/`Duration(unit)`) so the engine,
  not the model, decides which operations are category errors. `Dimension::combine(op, l, r)`
  encodes the strict algebra: **add/sub require matching dimensions** (`usd + eur`
  and `usd + days` are rejected — `usd + eur` will need a conversion fact in
  track A2); **`Money/Money → Scalar`** and `Unit(a)/Unit(a) → Scalar` (units
  cancel — the CSF:serum/debt-to-income ratio is dimensionless); `Money × Scalar
  → Money`, `× Percent` keeps the dimension; unlike dimensions multiply/divide to
  a composite tag the faithfulness gate can inspect.
- **`dimensioned_value(&Term)`** — generalises `numeric_magnitude` (step 2):
  reads the leading magnitude **and** infers the dimension from the wrapper
  functor (`money(18000, usd)` → `Money("usd")`, `quantity(40, mg_dl)` →
  `Unit("mg_dl")`, …). Tags are compared by equality, never interpreted (the
  engine knows `usd ≠ eur`, not that usd is dollars).
- `DimOp`, `DimError::Mismatch`, `Dimensioned`. This is the foundation for
  currency/date arithmetic (A2/A3) and the dimensional faithfulness gate (A4);
  `compute` stays numeric until A4 wires this in. See
  `code/specs/data/adj-language-expansion/ADJ-CONSTRAINTS-DESIGN.md`.

## [0.10.0] - 2026-06-11 — derivation tree (provenance-through-math, ADJ expansion step 3a)

### Added

- **`compute` module — the engine half of "the model never does the math".**
  A formula IR (`ComputeExpr`: `Ref(slot)` / `Lit(n)` / `Bin(op,a,b)` /
  `Agg(op,slot)`) is evaluated deterministically on the CPU into a `Derived`
  value carrying a **derivation tree** (`DerivationNode`): every operation
  records its operands and result, and every leaf cites the `FactId` of the
  observed fact it came from. So a derived value (`csf_ratio = csf_glucose /
  serum_glucose = 0.4`) is fully reconstructable from the tree without the
  model — provenance-through-math.
- `ComputeOp` — `Add/Sub/Mul/Div` (binary) and `Sum/Count/Min/Max/Avg`
  (aggregation over every observation of a slot). Operands read the magnitude
  of typed values (`quantity(40, mg_dl)`) via `numeric_magnitude`.
- `ComputeError` — clean, non-panicking errors (`UnknownSlot`,
  `EmptyAggregation`, `DivisionByZero`, `MalformedExpr`, plus two safety
  guards: `TooDeep` bounds recursion at `MAX_EVAL_DEPTH` so an adversarially
  deep formula returns an error instead of overflowing the stack, and
  `NonFinite` rejects any `NaN`/`±∞` result rather than letting it silently
  flow into a verdict — a `NaN` compares `false` against every threshold, so
  an unscreened non-finite would quietly make a predicate not fire).
- `KnowledgeBase::add_derived` / `derived_for`; `observed_value(slot)` now
  falls back to the derived table, so a **predicate-gated contribution fires
  over a computed value exactly as over an observed one** — one engine, no new
  verdict logic. New helpers `observed_value_with_fact` /
  `observed_values_all` expose the `FactId`(s) the derivation-tree leaves cite.
- A derived value can reference a previously-bound derived value (`let` over
  `let`) via a `DerivationNode::DerivedRef`.

This is engine-only (no surface syntax yet — `let name = expr` is step 3b). See
`code/specs/data/adj-language-expansion/STEP3-let-arithmetic-PLAN.md`.

## [0.9.0] - 2026-06-10 — typed-value magnitudes (ADJ language expansion, step 2)

### Added

- **`numeric_magnitude(&Term) -> Option<f64>`** — extract the numeric
  magnitude of a typed value. The ADJ language expansion models a fact's
  value as either a bare number or a *typed-value wrapper* carrying the
  magnitude as its leading argument and the unit afterward:
  `quantity(18000, usd)`, `money(18000, usd)`, `percentage(40)`,
  `duration(365, days)`, `count(3)`. The rule is uniform — "the leading
  numeric argument" — so no closed set of wrapper functors is hard-coded.

### Changed

- **`observed_value(slot)`** now reads through a typed-value wrapper via
  `numeric_magnitude`, so a predicate (`gross_income >= 14600`) fires over
  `observe gross_income(quantity(18000, usd))` while the `usd` unit stays
  attached to the fact for the (forthcoming) faithfulness gate. Bare
  `slot(Num)` facts behave exactly as before.

## [0.8.0] - 2026-06-10 — predicate-gated contributions (deterministic = saturating probabilistic)

### Added

- **`PredicateContributionClause` + `CmpOp`** — a likelihood-ratio
  contribution gated by a numeric comparison over a *valued slot*:
  "when the observed value of `slot` satisfies `slot <op> value`,
  multiply the conclusion's odds by `exp(logit_delta)`." This is the
  bridge that lets the framework express a **deterministic** rule as
  the saturating limit of a probabilistic one — a hard rule is just a
  very large LR over a CPU-evaluated predicate. DETERMINATE /
  INDETERMINATE / CONFLICT continue to fall out of the existing
  `differential` (leader / insufficient-evidence / kickback); there is
  **no second engine**.
- `CmpOp` — `Ge` / `Le` / `Gt` / `Lt` / `Eq` with `eval(lhs, rhs)`
  (the comparison the engine runs on the CPU) and `symbol()` (for
  audit rendering). `Eq` uses an absolute tolerance so an integer
  observation matches a float threshold.
- `KnowledgeBase::add_predicate_contribution`,
  `predicate_contributions_for`, and `observed_value(slot)` — the
  last reads the numeric value of the latest `Certain` valued fact
  `slot(V)` (V a `Term::Num`). Predicate clauses also count toward
  `participates_in_lr_aggregation`.
- `DerivationOrigin::FromPredicateContribution { clause_id, slot, op,
  threshold, observed, logit_delta }` — the proof step records the
  *literal* comparison that fired, so the audit trail shows the
  numbers the engine compared. The model never computes the
  comparison; it only authored the rule.

## [0.7.0] - 2026-06-10

### Added

- **`differential(hypotheses, kb)` — the cross-hypothesis decision
  primitive.** `lr_aggregate` scores one hypothesis at a time; the
  differential ranks a set of *competing* hypotheses (bacterial vs
  viral vs fungal meningitis, charge A vs B, deal-vs-no-deal), picks
  the argmax, and reports the **between-hypothesis margin**. This is
  the operation MYCIN actually performs and the engine previously
  lacked — nothing ranked competing conclusions or measured the gap.
- `DifferentialDecision` — `Determinate { leader, margin }` when the
  leader out-ranks the runner-up *even under the worst-case resolution
  of every open uncertainty* (leader's VOI band pushed down, runner-up's
  up); `Kickback { leader, runner_up, recommended_resolutions }` when
  the bands cross (an unresolved finding — or an exact tie — could flip
  the ranking). This is the cross-hypothesis analogue of
  `LRAggregateResult::suggest_kickback`, which only bounded a single
  hypothesis. Decision = argmax + sensitivity (ADJ65), deterministic and
  CPU-only — no softmax, no temperature.
- `RankedHypothesis` carries each hypothesis's full `LRAggregateResult`
  (proof DAG included) so the differential is auditable end to end, plus
  a `normalized_share` (posterior ÷ Σ posteriors) flagged as a
  display-only convenience that assumes the hypotheses are exhaustive and
  mutually exclusive (the LR model does not).
- Re-exported `differential`, `Differential`, `DifferentialDecision`,
  `RankedHypothesis` from the crate root.

## [0.6.0] - 2026-06-02

### Added

- `counterfactual(query, kb, &[Term])` — clones the KB, adds the
  given Facts as Certain, and reruns `lr_aggregate`. Lets the
  caller answer "what would the posterior be if X were true?"
  without disturbing the original KB. Cloning the whole KB makes
  the contract obvious; cost is linear and small.
- `LRAggregateResult::suggest_kickback(decision_threshold)` —
  computes a worst-case / best-case posterior band by reducing
  each active uncertainty marker to its min/max contribution,
  summing the shifts independently across markers, and applying
  to the current `posterior_logit`. Returns `Some(KickbackReport)`
  iff the band straddles `decision_threshold`. Includes
  `recommended_resolutions` sorted by individual VOI.
- `source_disagreements(kb, conclusion)` /
  `source_disagreements_with_threshold(kb, conclusion, min_spread)`
  — scans contributions on `conclusion`, groups by `evidence_term`,
  flags groups where the `logit_delta`s have spread > threshold.
  Per-source records include the clause id and provenance so the
  audit reader can render "AHA 2021 says LR=2.5; ESC 2023 says
  LR=4.0; sources disagree by 0.47 logits."
- New types: `KickbackReport`, `SourceDisagreementReport`,
  `SourceLogitDelta`.
- `KnowledgeBase: Clone`.
- 7 new tests covering counterfactual upward shift, KB
  non-mutation invariant, kickback firing inside the band, no
  kickback outside the band, source-disagreement detection on two
  conflicting sources, no-disagreement when only one source, and
  no-disagreement when sources agree.

Total tests: 69 (was 62 in 0.5.0).

### ADJ46 awkwardness items dissolved by 0.6.0

- **A7** (no kickback search variant) — addressed via
  `suggest_kickback` method on the result rather than a separate
  search mode. Lower-friction API and the same diagnostic power.
- **A8** (counterfactuals require KB clone + rerun) — `counterfactual`
  function does the clone + rerun once, atomically; caller's KB
  is invariant.
- **A9** (source-disagreement aggregation) — detector +
  per-source records surface conflicting LRs from the rulebook.

### Status of the original 10 ADJ46 awkwardness items

| Item | Status |
|---|---|
| A1 (LR magnitudes) | ✅ 0.3.0 |
| A2 (provenance) | ✅ 0.4.0 |
| A3 (prior) | ✅ 0.3.0 |
| A4 (joint contributions syntax) | ✅ 0.1.0 (adj-lang) |
| A5 (uncertainty markers) | ✅ 0.5.0 |
| A6 (WMC vs LR) | ✅ 0.3.0 |
| A7 (kickback) | ✅ 0.6.0 |
| A8 (counterfactuals) | ✅ 0.6.0 |
| A9 (source disagreement) | ✅ 0.6.0 |
| A10 (surface syntax) | ✅ 0.1.0 (adj-lang) |

All ten items dissolved as of logic-engine 0.6.0 +
adj-lang 0.2.0.

## [0.5.0] - 2026-06-02

### Added

- `UncertaintyMarker` clause type + `UncertaintyMarkerId`. Attached
  to a conclusion with a `domain: Vec<Term>` of candidate evidence
  terms. Represents "the IR pipeline knows the conclusion is the
  target of an LR query, and knows the patient (or source) did not
  specify one of these candidate values."
- `UncertaintyReport` — the user-facing VOI summary the engine
  emits when a marker's domain is entirely unobserved. Contains the
  domain, the log-odds delta each value would have contributed if
  observed, and a v0.1 VOI proxy (`voi_logit_range` = max − min of
  the deltas). The framework's user-facing layer can rank these to
  produce "if you can determine X, the posterior could swing by up
  to Y" guidance.
- `LRAggregateResult.uncertainties: Vec<UncertaintyReport>` +
  `SearchResult::LRAggregateResult { ..., uncertainties }`.
- `KnowledgeBase::add_uncertainty_marker` /
  `uncertainty_markers_for`. Markers do not promote a query to
  LR-aggregation — they're only meaningful relative to contribution
  clauses already on the conclusion.
- 3 new integration tests in `tests/test_lr_aggregation.rs`:
  uncertainty report with no observation shows full domain + VOI,
  one-domain-observation suppresses the report, marker over a
  domain with no matching contributions has zero VOI but still
  appears in the report.

Total tests: 62 (was 59 in 0.4.0).

### ADJ46 awkwardness items dissolved by 0.5.0

- **A5** — uncertainty markers at the engine layer.
  `add_uncertainty_marker` + `UncertaintyReport` give the IR
  pipeline a way to losslessly hand off "the patient said nothing
  about X over this domain" to the executor, and give the audit
  reader a concrete VOI signal to act on.

### Scope notes

- VOI is the v0.1 proxy (max − min over candidate log-odds deltas)
  — not the formal Bayesian decision-theoretic VOI. A richer
  treatment that combines the candidate deltas with the prior over
  the domain (and with the user's decision threshold, if any) is a
  follow-up.
- Still pending: A7 (kickback variant), A8 (counterfactuals), A9
  (multi-source aggregation), and the surface-layer half of A5
  (the `uncertain { ... } for ...` keyword — that ships in
  `adj-lang` 0.2.0 simultaneously).

## [0.4.0] - 2026-06-02

### Added

- `provenance` module: `Provenance { source, locator, trust_tier }`
  + `TrustTier { Consensus, Authoritative, Empirical, Inferred,
  Unattributed }`. Designed so the common case is a one-liner —
  `Provenance::cited("AHA 2021 §3.2")` — while still carrying enough
  structure that an audit reader can sort or filter across clauses
  by trust tier.
- `PriorClause`, `ContributionClause`, `JointContributionClause`
  each grow a `provenance: Provenance` field plus a
  builder-style `.with_provenance(...)` method. Default is
  `Provenance::unattributed()` so existing pre-ADJ47-B code
  continues to construct clauses without any source-of-truth
  ambiguity.
- 5 new inline unit tests in `provenance.rs` covering trust-tier
  ordering, locator builder-style threading, and the default.
- 2 new integration tests in `tests/test_lr_aggregation.rs`:
  `provenance_is_recoverable_from_kb_after_aggregation` (the
  contract: clauses carry citations and the audit reader recovers
  them via the clause id from the proof DAG, no side-table) and
  `unattributed_provenance_is_the_default` (legacy compatibility).

Total tests: 59 (was 52 in 0.3.0).

### ADJ46 awkwardness items dissolved by 0.4.0

- **A2** (provenance is not a clause field) — fully addressed.
  Clauses now carry citations; the proof DAG references them by
  clause id; no side-table required.

### Scope notes

What 0.4.0 still does NOT ship: A4 (joint as syntactically
distinct from atomic in the *surface* syntax — semantically the
engine already distinguishes them), A5 (uncertainty markers), A7
(kickback search variant), A8 (counterfactuals), A9 (source-
disagreement aggregation, though `Provenance` is the prerequisite
data structure), A10 (surface syntax) — all language-layer.

## [0.3.0] - 2026-06-02

### Added

- `lr_aggregate` module: full implementation of
  [`LP19e`](../../../specs/LP19e-likelihood-ratio-aggregation.md)
  likelihood-ratio Bayesian aggregation. Three new clause types
  (`PriorClause`, `ContributionClause`, `JointContributionClause`)
  plus three new id types, an `lr_aggregate(query, kb)` function,
  numerically stable `sigmoid` / `logit` helpers, an
  `LRAggregateResult` carrying the proof DAG and posterior, and an
  `LrAggregateWarning` enum surfacing the LP19e §"Edge cases"
  (no prior declared, no contributions active, degenerate LR=1.0
  contribution).
- `SearchMode::LRAggregate` variant + `SearchResult::LRAggregateResult`
  variant. `AutoDetect` now routes to `LRAggregate` first whenever
  `kb.participates_in_lr_aggregation(query)` is true, then falls
  back to the LP19 short-circuit between `FindFirst` and
  `EnumerateAll`.
- `KnowledgeBase` extensions: `add_prior`, `add_contribution`,
  `add_joint_contribution`, `prior_for`, `contributions_for`,
  `joint_contributions_for`, `participates_in_lr_aggregation`,
  `observed_evidence`. The new storage is flat `Vec`s rather than
  `HashMap<Term, _>` because `Term` does not implement
  `Hash + Eq`; linear scan is fine at current scale and switching
  to an indexed map later is purely additive.
- `DerivationOrigin` grows three additive variants: `FromPrior`,
  `FromContribution`, `FromJointContribution`. Each carries the
  log-odds delta inline so an audit reader can reconstruct running
  log-odds from the proof's `steps` without consulting the KB.
- `Proof` grows two additive fields: `posterior_logit:
  Option<f64>` and `posterior_probability: Option<f64>`. `Some(_)`
  on LR-aggregation proofs, `None` on SLD / WMC proofs.
- 7 integration tests in `tests/test_lr_aggregation.rs` covering
  the ADJ36 ACS chest-pain scenario end-to-end (reproduces 28.1%
  posterior), `AutoDetect` routing, missing-prior warning, joint
  contributions, evidence Fact id threading into the proof DAG,
  compound-term equality on the linear-scan lookup, and conflicting
  priors rejection.
- 9 inline unit tests in `lr_aggregate.rs` covering numeric
  stability, round-trip through `logit`/`sigmoid`, constructor
  panics on out-of-range inputs, the prior-only case, single and
  joint contributions, unobserved evidence skipped, and
  `KbError::ConflictingPriors`.

Total tests: 52 (was 36 in 0.2.0).

### Scope notes

This slice dissolves the engine-layer half of ADJ46's awkwardness
catalogue at items A1 (LR magnitudes), A3 (Bayesian prior), A6 (WMC
discarded; we now compute the right posterior), and starts on A2
(provenance — id types are now distinct so the audit trail can name
the clause kind, though source-citation fields on clauses themselves
are still ADJ47 follow-up work).

What 0.3.0 does NOT yet ship: counterfactual queries (A8),
source-disagreement aggregation (A9), uncertainty markers (A5),
kickback variant (A7), or a surface syntax (A10) — all are language-
layer and live in ADJ47.

## [0.2.0] - 2026-05-11

### Added

- `proof_dag` module: `ProofDAG`, `Proof`, `ProofStep`, `DerivationOrigin`
  — the engine's return type when enumeration is active. Each `Proof`
  records its final substitution, an ordered list of derivation steps,
  and de-duplicated `via_facts` / `via_rules` lists that name every
  probabilistic clause the proof depends on.
- `enumerate` module: `enumerate_all(query, kb)` — exhaustive SLD that
  collects every successful derivation rather than stopping at the
  first. Uses the same fresh-variable renaming as `find_first` so that
  multiple clause instantiations don't share variable identity.
  Negation-as-failure is the well-founded reading per LP19.
- `wmc` module: `weighted_model_count(dag, kb)` — naïve enumeration
  over `2^n` worlds, where `n` is the count of distinct probabilistic
  clauses across all proofs. Certain clauses are automatically true
  and do not contribute degrees of freedom. The shared-fact case is
  handled correctly because WMC counts worlds, not paths.
- `SearchResult` enum with `FindFirstResult` and `EnumerateAllResult`
  variants, plus a top-level `search(query, kb, mode)` function. In
  `AutoDetect` mode the engine inspects `kb.is_all_certain()` and
  selects `FindFirst` when every clause is `Certain` — the LP19
  short-circuit theorem made executable.
- `KnowledgeBase::find_fact_by_id` and `find_rule_by_id` — linear-scan
  lookup used by the WMC backend to recover Bernoulli parameters from
  clause ids. Sufficient at current scale; an indexed alternative may
  arrive in a later slice.
- 11 new tests (4 enumerate, 7 wmc, 4 integration) including the
  canonical `P(path(a,c)) = 0.86` graph reachability and the
  shared-fact case that fails under naïve inclusion-exclusion (correctly
  returns 0.5 here, would be 0.75 under the wrong algorithm). Total:
  30 tests.

### Scope notes

This slice completes the probabilistic core specified in
[`LP19`](../../../specs/LP19-probabilistic-logic-core.md) for the naïve
inference path. `d-DNNF` / `SDD` compilation (LP19a), rational
arithmetic (LP19b), conditional probability with evidence (LP19c), and
approximate inference (LP19d) remain as planned follow-ups.

## [0.1.0] - 2026-05-11

### Added

- `Probability` enum with two variants: `Certain` (semantic 1.0, recognized
  structurally for the LP19 short-circuit) and `Value(f64)` for genuine
  probabilities in `[0, 1]`.
- `Fact { id, term, probability }` carrying a stable `FactId` and a
  probability that defaults to `Certain`.
- `Rule { id, head, body, probability }` with `BodyLiteral::{Pos, Neg}`
  body literals (positive goals and negation-as-failure).
- `KnowledgeBase` — an indexed collection of Facts and Rules. Looks up
  by head functor/arity for fast clause selection during search.
- `SearchMode` enum: `FindFirst`, `EnumerateAll`, `AutoDetect`.
- `is_all_certain()` on `KnowledgeBase` — the precondition for the
  LP19 short-circuit. The implementation walks every Fact and Rule once
  and is `O(|KB|)`.
- `find_first(query, kb)` — deterministic SLD-style resolution over the
  KB. Returns the first successful `Substitution` or `None`. Uses the
  unification from `logic-core` and the KB's clause index for clause
  selection.
- 14 tests covering: deterministic facts, rules with bodies, multiple
  clauses with backtracking, the all-Certain short-circuit detection,
  rejection of anonymous probabilistic clauses (well-formedness), and a
  small "family relations" worked example used by the LP layer's
  educational specs.

### Scope

This is the first slice of [`LP19`](../../../specs/LP19-probabilistic-logic-core.md).
Subsequent slices will add:

- Proof DAG construction (return all successful derivations, not just
  the first).
- `EnumerateAll` and `AutoDetect` mode implementations.
- Naïve weighted-model-counting backend over the proof DAG's induced
  Boolean formula.
- d-DNNF / SDD compilation (LP19a).

The current slice deliberately limits itself to **deterministic
find-first search** so that the foundation can be reviewed before the
probabilistic backend is added.

### Notes

The Python reference at `code/packages/python/logic-engine` remains the
canonical interpretation. The Rust crate currently is a strict subset
of the Python API surface; subsequent PRs expand to parity.

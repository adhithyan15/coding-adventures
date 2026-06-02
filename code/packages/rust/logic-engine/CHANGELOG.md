# Changelog

All notable changes to this project will be documented in this file.

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

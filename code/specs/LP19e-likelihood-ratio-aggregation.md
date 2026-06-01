# LP19e — Likelihood-Ratio Aggregation: Bayesian Posterior Inference

## Overview

[`LP19`](LP19-probabilistic-logic-core.md) defines exact weighted
model counting over possible worlds. That algorithm is the right
answer for **joint conjunctive probabilistic programs** — where a
clause body fires iff every literal in the body is independently
true, and the answer is a sum over possible-worlds probability mass.

[`ADJ14`](ADJ14-probabilistic-ir-semantics.md) commits the
adjudication framework to a *different* dominant inference shape:
**independent evidence atoms each contributing a likelihood ratio to
a conclusion, composed multiplicatively in log-odds space**. This is
how evidence-based medicine, Bayesian forensic statistics, and
quantitative legal-sufficiency review actually reason. It is also
the inference shape the framework's small-model + retry-with-correction
thesis depends on, because it makes the proof DAG natively
human-readable: a verdict is a *prior plus a list of named
contributions*, not a sum over enumerated possible worlds.

This sub-spec specifies the engine's algorithm for that shape.

## Why a separate sub-spec

The math is straightforward. Specifying it explicitly matters because
the API surface, the proof-DAG shape, and the mode-selection
interaction with `AutoDetect` all have to be nailed down before the
connector (ADJ11 v2) can lower `contributes` / `prior` clauses, and
before the audit trail (ADJ15) can serialize LR contributions as
first-class proof steps.

LP19e is the engine engineering deliverable that makes ADJ14
executable.

## Layer position

```
   LP19  probabilistic logic core         ← exact WMC over proof DAG
        │
        ├── LP19a  d-DNNF compilation     ← scales WMC formula side
        ├── LP19b  rational arithmetic    ← scales WMC precision side
        ├── LP19c  conditional probability ← P(query | evidence) via WMC ratio
        ├── LP19d  approximate inference  ← Monte Carlo over WMC's worlds
        └── LP19e  LR aggregation         ← THIS SPEC: log-odds composition
                                            for Bayesian inference
```

LP19e is purely additive. Existing `SearchMode::FindFirst`,
`EnumerateAll`, `AutoDetect` modes continue to work unchanged. A new
`SearchMode::LRAggregate` variant joins them, and `AutoDetect` learns
to route to it.

## The new clause types

ADJ14 specifies the **user-facing rule subtypes** (`prior`,
`contributes`, `contributes_jointly`) that the connector recognises.
This spec defines the **engine-internal clause types** that result
from lowering them.

```rust
/// A prior probability for a conclusion. Required exactly once per
/// conclusion that participates in LR aggregation. Multiple priors
/// for the same conclusion is a KB-construction error
/// (`KbError::ConflictingPriors`).
#[derive(Debug, Clone, PartialEq)]
pub struct PriorClause {
    pub id: PriorClauseId,
    pub conclusion: Term,
    pub prior_logit: f64,      // log(p / (1 - p))
}

/// A single-source likelihood-ratio contribution. Multiple
/// contributions per (conclusion, evidence_term) pair are permitted
/// — they sum in log-odds — and used for cross-rulebook composition.
#[derive(Debug, Clone, PartialEq)]
pub struct ContributionClause {
    pub id: ContributionClauseId,
    pub conclusion: Term,
    pub evidence_term: Term,
    pub logit_delta: f64,      // log(LR)
}

/// A joint-evidence interaction term. Active iff every term in
/// evidence_set is observed in the current KB. Synergy if
/// joint_logit_delta > 0; suppression / explaining-away if < 0.
#[derive(Debug, Clone, PartialEq)]
pub struct JointContributionClause {
    pub id: JointContributionClauseId,
    pub conclusion: Term,
    pub evidence_set: Vec<Term>,
    pub joint_logit_delta: f64,
}
```

Each new clause type carries a fresh-typed id so it can be cited in
proof-DAG steps and in the lowering map (ADJ15).

## The KB extension

`KnowledgeBase` grows three additive parallel maps (none break
existing Fact / Rule storage):

```rust
pub struct KnowledgeBase {
    // …existing fact and rule indexes…

    priors:        HashMap<Term, PriorClause>,                       // new
    contributions: HashMap<Term, Vec<ContributionClause>>,           // new
    joint_contributions: HashMap<Term, Vec<JointContributionClause>>,// new
}
```

Three new accessors:

```rust
impl KnowledgeBase {
    pub fn prior_for(&self, conclusion: &Term) -> Option<&PriorClause>;
    pub fn contributions_for(&self, conclusion: &Term) -> &[ContributionClause];
    pub fn joint_contributions_for(&self, conclusion: &Term) -> &[JointContributionClause];

    pub fn add_prior(&mut self, clause: PriorClause) -> Result<(), KbError>;
    pub fn add_contribution(&mut self, clause: ContributionClause);
    pub fn add_joint_contribution(&mut self, clause: JointContributionClause);

    /// True iff at least one contribution names `conclusion` as its target.
    pub fn participates_in_lr_aggregation(&self, conclusion: &Term) -> bool;
}
```

`add_prior` is the only one that can fail — `ConflictingPriors` if a
prior for the same conclusion already exists.

## The new SearchMode variant

```rust
pub enum SearchMode {
    FindFirst,
    EnumerateAll,
    AutoDetect,
    LRAggregate,    // new in LP19e
}
```

And a new result variant:

```rust
pub enum SearchResult {
    FindFirstResult(Option<Substitution>),
    EnumerateAllResult { dag: ProofDAG, probability: f64 },
    LRAggregateResult { dag: ProofDAG, posterior: f64, posterior_logit: f64 },
        // new in LP19e
}
```

`LRAggregateResult.dag` carries one `Proof` whose `steps` enumerate
the prior, every active contribution, and every active joint
contribution — in evaluation order.

## The inference algorithm

```rust
fn lr_aggregate(query: &Term, kb: &KnowledgeBase) -> LRAggregateResult {
    // 1. Require a prior. Without it, we cannot start.
    let prior = match kb.prior_for(query) {
        Some(p) => p,
        None => {
            // No prior → fall through with logit = 0 (P = 0.5), warned.
            return LRAggregateResult::no_prior(query.clone());
        }
    };

    let mut steps = vec![ProofStep {
        goal: query.clone(),
        origin: DerivationOrigin::FromPrior {
            clause_id: prior.id,
            prior_logit: prior.prior_logit,
        },
    }];
    let mut logit = prior.prior_logit;
    let mut via_facts: Vec<FactId> = Vec::new();
    let mut via_rules: Vec<RuleId> = Vec::new();

    // 2. Apply every active single-source contribution.
    for contrib in kb.contributions_for(query) {
        if let Some(observed_fact_ids) = kb.observed_evidence(&contrib.evidence_term) {
            steps.push(ProofStep {
                goal: query.clone(),
                origin: DerivationOrigin::FromContribution {
                    clause_id: contrib.id,
                    evidence_fact_ids: observed_fact_ids.clone(),
                    logit_delta: contrib.logit_delta,
                },
            });
            logit += contrib.logit_delta;
            via_facts.extend(observed_fact_ids);
        }
    }

    // 3. Apply every active joint contribution.
    for joint in kb.joint_contributions_for(query) {
        let mut all_observed = true;
        let mut every_evidence: Vec<FactId> = Vec::new();
        for ev_term in &joint.evidence_set {
            match kb.observed_evidence(ev_term) {
                Some(ids) => every_evidence.extend(ids),
                None => { all_observed = false; break; }
            }
        }
        if all_observed {
            steps.push(ProofStep {
                goal: query.clone(),
                origin: DerivationOrigin::FromJointContribution {
                    clause_id: joint.id,
                    evidence_fact_ids: every_evidence,
                    joint_logit_delta: joint.joint_logit_delta,
                },
            });
            logit += joint.joint_logit_delta;
        }
    }

    // 4. Convert to posterior and emit.
    let posterior = sigmoid(logit);
    LRAggregateResult {
        dag: ProofDAG {
            root_query: query.clone(),
            proofs: vec![Proof {
                bindings: Substitution::empty(),
                steps,
                via_facts: dedup(via_facts),
                via_rules,
                posterior_logit: Some(logit),
                posterior_probability: Some(posterior),
            }],
        },
        posterior,
        posterior_logit: logit,
    }
}

fn sigmoid(x: f64) -> f64 {
    if x >= 0.0 {
        let z = (-x).exp();
        1.0 / (1.0 + z)
    } else {
        // Numerically stable for very negative x: avoids exp(huge) overflow.
        let z = x.exp();
        z / (1.0 + z)
    }
}
```

**Complexity**: linear in the number of contributions and joint
contributions naming the query. The 2ⁿ-world enumeration that WMC
needs is *not* required for this inference shape — the
conditional-independence assumption makes the math collapse to a sum.

## AutoDetect: extended routing

`AutoDetect` chooses between `FindFirst`, `EnumerateAll`, and
`LRAggregate` per query. The decision tree:

```text
                  query: Q
                     │
                     ▼
    ┌───────────────────────────────────┐
    │ kb.participates_in_lr_aggregation(Q) ?
    └───────────────────────────────────┘
            │ yes                  │ no
            ▼                      ▼
    ┌─────────────┐    ┌───────────────────────┐
    │ LRAggregate │    │ kb.is_all_certain() ? │
    └─────────────┘    └───────────────────────┘
                              │ yes        │ no
                              ▼            ▼
                       ┌──────────┐  ┌──────────────┐
                       │ FindFirst│  │ EnumerateAll │
                       └──────────┘  └──────────────┘
```

Per ADJ14, the connector raises `MixedShapeOnSameConclusion` when a
query is the target of *both* `contributes(*, *, Q)` and
`probabilistic(p, Q, …)` — i.e., the user has declared both
inference shapes for the same conclusion. The engine never sees this
case (the connector rejects it at lowering time), but the routing
code asserts it as an invariant for defense in depth.

## Observed evidence — what counts as "observed"

The algorithm above calls `kb.observed_evidence(term) -> Option<Vec<FactId>>`.
Its semantics:

- A `Fact` clause with this term, polarity = Affirmed, and
  `Probability::Certain`: returns the fact's id.
- A `Fact` clause with this term and polarity = Affirmed but
  `Probability::Value(p)`: the fact is *probabilistically observed*.
  For LR aggregation, the spec's v0.1 treats this as observed with
  full weight (the probability-of-observation enters separately via
  the extractor-LR — see ADJ14 §"Reinterpreting IRNode::confidence");
  v0.2 will route to a more careful treatment.
- A `Fact` with polarity = Denied: **not observed**. The
  contribution is not applied. A separate `contributes(LR, ¬e, c)`
  clause is the right way to model "absence of evidence is evidence
  of absence."
- No matching fact in the KB: not observed. Skipped.

This deliberately keeps the "observation gate" mechanical and
inspectable. Adding latent variables is a future-work item.

## Proof DAG integration

`Proof` grows two additive fields:

```rust
pub struct Proof {
    pub bindings: Substitution,
    pub steps: Vec<ProofStep>,
    pub via_facts: Vec<FactId>,
    pub via_rules: Vec<RuleId>,
    pub posterior_logit: Option<f64>,        // new
    pub posterior_probability: Option<f64>,  // new
}
```

Both are `Option<f64>`:
- `Some(_)` after an `LRAggregate` search.
- `None` after `FindFirst`, `EnumerateAll`, or `AutoDetect → WMC`.

`DerivationOrigin` grows three additive variants (already specified
in ADJ14):

```rust
pub enum DerivationOrigin {
    FromFact(FactId),
    FromRule(RuleId),
    FromPrior { clause_id: PriorClauseId, prior_logit: f64 },
    FromContribution {
        clause_id: ContributionClauseId,
        evidence_fact_ids: Vec<FactId>,
        logit_delta: f64,
    },
    FromJointContribution {
        clause_id: JointContributionClauseId,
        evidence_fact_ids: Vec<FactId>,
        joint_logit_delta: f64,
    },
}
```

A reviewer following the proof DAG reads the prior, then every
contribution in evaluation order, with running logit reconstructible
from the deltas. The DAG is the derivation.

## Audit trail integration

`adjudication-audit-trail::EngineArtifacts` grows three additive
fields per ADJ14:

```rust
pub struct EngineArtifacts {
    // …existing fields…
    pub independence_assumption_used: bool,
    pub lr_contributions: Vec<LrContributionRecord>,
    pub prior_record: Option<PriorRecord>,
}
```

Every `LrContributionRecord` carries:
- the clause id and IR node id (via the lowering map of ADJ15),
- the evidence facts' ids and IR node ids,
- the LR value, the logit delta, and the running logit *after* this
  contribution,
- the source-span citations the proof is grounded in.

The trail records the full computation, not the conclusion.

## Edge cases

### No prior declared
The query falls through with `prior_logit = 0` (uniform). The result
sets `prior_record = None` and adds a `Warning` to the audit trail.
This is the only path the engine produces a "default" probability;
explicit priors are encouraged.

### No contributions active
The result is the prior (i.e., `posterior == sigmoid(prior_logit)`).
The proof DAG contains exactly one step (`FromPrior`). The
derivation rendering surfaces this honestly: "no observed evidence
contributes; verdict equals prior."

### A contribution names an evidence atom that's not in the KB
Silently skipped. Not an error — it's the common case when an
LR table covers more symptoms than any one patient presents.

### A `contributes(LR=1.0, …)` clause
Permitted but a no-op. The engine emits a `Warning` once per such
clause to surface likely modeler intent errors.

### A negative or zero LR
`LR ≤ 0` is rejected at clause construction. Negative log-odds are
modeled with a positive LR less than 1 (e.g., LR=0.5 means the
evidence halves the odds), per Bayesian convention.

### Numerical stability
Logit arithmetic is performed in `f64` with a final `sigmoid` step.
For extreme logit values (`|logit| > 700`) the sigmoid saturates to
0 or 1 by IEEE-754 rules — acceptable for the v0.1 implementation,
with no quiet error. Deployments needing more precision can route
through LP19b (rational arithmetic), which gets a future
extension to support log-odds rationals.

## Mode interaction with conditional probability (LP19c)

`search_conditional(query, evidence, kb, mode)` is the LP19c API for
conditional probability. With `mode = LRAggregate`:
- Every observation in `evidence` is treated as an "observed" fact
  for the purpose of `contributions_for(query)` matching.
- The prior is unaffected by the evidence (priors are prior).
- The contributions for the conclusion are filtered by which of
  their `evidence_term`s are present in the conditioning set.

This composition makes LP19c a perfect orthogonal layer on top of
LP19e: declare evidence in the LP19c surface, get the LR-aggregated
posterior automatically. Same proof DAG structure, same audit trail
shape.

## What's deliberately out of scope

- **Learning LRs from labelled outcome data.** That belongs in
  ADJ09 (rule compilation) or a future ADJ-calibration sub-spec.
- **Drift detection / recalibration over time.** Same answer.
- **Hierarchical priors** (a prior conditional on demographic
  bucket, etc.). Representable today as one `prior(p, c)` per
  bucket and a `contributes` from a demographic fact, but a
  dedicated hierarchical primitive is a future addition.
- **Soft observations** (an observation reported with its own
  probability). v0.1 treats observation as Boolean; the
  extractor-LR pathway in ADJ14 covers the LLM-confidence case.
  Generalised soft observations are future work.

## Open questions

1. **How to fail loud on missing priors.** Default to uniform with
   a warning, or refuse to run with `LRMissingPrior`? v0.1 says
   warn-and-proceed; v0.2 may flip to refuse-by-default with a
   configuration flag, as missing priors are a common rulebook
   construction bug.
2. **Multi-conclusion queries.** A clinical query may ask for the
   posterior on *every* candidate diagnosis (the differential).
   The current API runs one query at a time; a batched
   `lr_aggregate_all(kb)` would compute every targeted conclusion
   in one pass and could share evidence-presence checks. Performance
   optimization; design in v0.2.
3. **Cycles in joint contributions.** A `contributes_jointly([a, b],
   c)` is straightforward. A `contributes_jointly([a, c], a)` is
   not — the conclusion appears in its own evidence set. v0.1
   rejects this at clause construction; v0.2 may consider
   fixed-point semantics if a real use case appears.

## Limitations

1. **The independence assumption is not eliminated by joint terms.**
   Joint contributions partially absorb correlation, but a fully
   correlated set of evidence atoms still requires careful modeling.
   The audit trail records the assumption as used; the modeler is
   responsible for whether it's appropriate.
2. **`contributes` clauses are flat.** Hierarchical contribution
   (e.g., "abdominal symptoms" as an umbrella term that
   `gastritis` and `appendicitis` contribute under) is not natively
   modeled. A representation via `definitional` rules linking
   abstract terms to ground evidence works today.
3. **Engine numerical precision is f64.** Extreme priors or
   long chains of contributions can saturate the sigmoid. LP19b
   (rational) provides the path to higher precision when needed.

## Status

Draft. Implementation is straightforward and self-contained inside
`logic-engine`: new clause types, new KB indexes, new SearchMode
variant, new ProofDAG fields. No breaking changes to existing APIs.

The connector's v2 (ADJ11 v2) is the integration deliverable that
makes ADJ14's user-facing rule subtypes flow through this engine.

## Where to read next

- [ADJ14](ADJ14-probabilistic-ir-semantics.md) — the user-facing
  semantics this sub-spec implements.
- [LP19](LP19-probabilistic-logic-core.md) — the engine foundation
  this sub-spec extends.
- [LP19c](LP19c-conditional-probability-evidence.md) — the
  conditional-probability layer that composes with this one.
- [ADJ15](ADJ15-lowering-map-and-proof-dag.md) — the audit-trail
  integration that surfaces this sub-spec's outputs.

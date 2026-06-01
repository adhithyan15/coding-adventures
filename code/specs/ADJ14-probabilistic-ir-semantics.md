# ADJ14 — Probabilistic IR Semantics: Likelihood-Ratio Aggregation

> The spec that decides what "probabilistic adjudication" actually
> means in this framework. Commits the IR + engine to a Bayesian
> likelihood-ratio aggregation inference model — the math working
> clinicians, lawyers, and analysts already reason in — and lays out
> the grammar, lowering rules, engine algorithm, and proof-DAG
> changes that make it real.

## Overview

The adjudication framework's design principle has been "the IR forces
the model to commit." Through ADJ01–ADJ13 this commitment was
*structural*: every byte belongs to a typed node; every node carries
polarity / modality. ADJ14 extends the commitment to be *probabilistic*:
every contribution of evidence to a conclusion is recorded as a
likelihood ratio, posterior probabilities are computed in log-odds
space, and the proof DAG ships every LR contribution as a first-class
derivation step.

This is the right inference model because the dominant real-world
adjudication tasks — clinical differential diagnosis, legal
sufficiency-of-evidence review, fraud risk scoring, intelligence
analysis, scientific peer review — all follow the same conceptual
pattern: **a base-rate prior, updated by independent (or
quasi-independent) pieces of evidence, each shifting the posterior
odds by a multiplicative factor.** Evidence-based medicine teaches
this explicitly (positive likelihood ratio LR+, negative likelihood
ratio LR−, pre-test and post-test odds); Bayesian forensic statistics
formalizes it the same way (the "likelihood-ratio framework"). The
framework adopts the same vocabulary.

## The decision, in one paragraph

The framework treats every assertion about an underlying state — an
extractor's claim that a symptom is present, a rulebook's claim that
a symptom raises a diagnosis's probability, a witness's testimony
about a contract clause — as a **likelihood ratio on the underlying
state, composed multiplicatively in log-odds space**. Conjunctive
ProbLog-style rules (where the rule fires iff every body literal is
proved) remain available for the cases where they're the right model
(deterministic constraints, definitional rules), but the
**probabilistic default is LR aggregation, not joint conjunction**.
A single field — likelihood ratio — replaces the prior distinction
between *epistemic* uncertainty (the model isn't sure what the source
said) and *aleatoric* uncertainty (the world is genuinely random):
both compose by the same arithmetic.

## Layer position

```
        ADJ01 (IR grammar v2)
              │
              ▼
        ADJ14 (THIS SPEC — probabilistic semantics)
              │
              ├──▶ ADJ11 v2 (connector: lowers contributes/prior subtypes)
              │
              ▼
        LP19e (engine: LR-aggregation inference mode)
              │
              ▼
        ADJ15 (proof DAG in audit trail — typed, includes LRContribution)
              │
              ▼
        ADJ16 (human-readable derivation rendering)
```

ADJ14 stacks on ADJ01 v2 and depends on a new engine sub-spec
**LP19e** (the LR-aggregation inference algorithm in `logic-engine`),
which is the engineering deliverable that makes this spec executable.
ADJ11 grows a v2 that recognises the two new rule subtypes; the
existing `definitional` / `constraint` / `default` subtypes are
unchanged.

## What this spec defines

1. **Two new `Rule` subtypes** (`contributes` and `prior`), plus an
   optional third (`contributes_jointly`) for interaction terms.
2. **Reinterpretation of `IRNode::confidence`** as a calibrated
   probability of observation-correctness, which lowers to an
   extractor-level likelihood ratio.
3. **Lowering rules** for `NodeKind::Uncertainty` and for `Fact`
   nodes with non-unit confidence.
4. **LR-aggregation inference algorithm** in log-odds space, with
   conditional-independence assumptions made explicit.
5. **A new `DerivationOrigin::FromContribution` variant** so the
   proof DAG carries every LR step as first-class data.
6. **Compatibility rules** between LR-aggregation queries and
   existing ProbLog-style WMC queries inside the same KB.

What this spec does **not** define: cloud-LLM-based extractor
calibration (ADJ14b candidate), interaction-term discovery / learning
(deferred), or the human-readable derivation renderer (ADJ16).

## Vocabulary

| Term | Symbol | Definition |
|---|---|---|
| Prior odds | `O₀(c)` | `P(c) / (1 − P(c))` before any evidence |
| Posterior odds | `O(c \| E)` | Prior odds × ∏ LR for observed evidence |
| Likelihood ratio | `LR(e, c)` | `P(e \| c) / P(e \| ¬c)` for one evidence atom |
| Prior logit | `λ₀(c)` | `log O₀(c)` |
| Posterior logit | `λ(c \| E)` | `λ₀(c) + Σ log LR(eᵢ, c)` |
| Posterior probability | `P(c \| E)` | `σ(λ(c \| E))` = `1 / (1 + e^{−λ})` |

All math happens in log-odds (logit) space. This is numerically
stable, makes contributions additive, makes LRs interpretable
("LR=10 means this evidence makes the diagnosis 10× more likely"),
and matches how evidence-based medicine and Bayesian forensics
teach the math.

## Grammar — new Rule subtypes

ADJ Rule nodes encode their subtype in `term` (per ADJ01 convention).
ADJ14 adds two required subtypes and one optional:

### `prior(P, conclusion)` — base rate

```text
prior(p, c)  where  0 ≤ p < 1
```

Establishes the base rate (prior probability) for conclusion `c`.
Required exactly once per conclusion that participates in LR
aggregation. Multiple `prior` declarations for the same conclusion
are a lowering error (`ConflictingPriors`).

**Lowering**: produces an internal `PriorClause { conclusion: c,
prior_logit: log(p / (1 − p)) }` recorded in the KB.

### `contributes(LR, evidence, conclusion)` — single-source contribution

```text
contributes(lr, e, c)  where  lr > 0,  lr ≠ 1.0  by convention
```

Observing `e` shifts the posterior odds of `c` by a multiplicative
factor of `lr`. `lr > 1` is a positive contributor (LR+); `lr < 1` is
a negative contributor (LR−). `lr = 1` is permitted but semantically
a no-op and is warned about at lowering time.

**Lowering**: produces an internal `ContributionClause { conclusion:
c, evidence_term: e, logit_delta: log(lr) }`.

**Independence assumption**: contributions to the same conclusion are
treated as conditionally independent given the conclusion (the
classic "naïve Bayes" assumption). The assumption is explicit, not
implicit; ADJ14 surfaces it in the audit trail as
`engine_artifacts.independence_assumption_used: true` whenever LR
aggregation produced the verdict.

### `contributes_jointly(LR_extra, [e₁, …, eₙ], conclusion)` — interaction term

```text
contributes_jointly(lr_extra, [e1, e2, …, en], c)   where lr_extra > 0
```

When *all* of `[e₁, …, eₙ]` are observed simultaneously, apply an
**additional** multiplicative factor `lr_extra` on top of each
individual `contributes` contribution. `lr_extra > 1` models synergy
("these three findings together raise P(diagnosis) more than the
product of their individual LRs would suggest"); `lr_extra < 1`
models suppression / explaining-away.

**Lowering**: produces `JointContributionClause { conclusion: c,
evidence_set: {e₁,…,eₙ}, joint_logit_delta: log(lr_extra) }`.

Optional in v0.1 of the spec — the LR-aggregation algorithm handles
the absence of any `contributes_jointly` clause as the pure
conditional-independence case.

## Reinterpreting `IRNode::confidence`

In ADJ01 v2 the field reads:
> Extractor's self-reported confidence. Informational only; not used
> by the type check.

ADJ14 makes it semantic:

> Extractor's self-reported probability that the observation it
> emitted faithfully represents the source. Lowers to an
> extractor-level likelihood ratio.

**Lowering rule for a `Fact` node with `confidence = c`**:

- If `c ≥ 1.0 − ε` (configurable, default `ε = 0.05`): treat as
  `Probability::Certain`. The observation is asserted.
- Otherwise: lower to an *observed* atom plus an implicit
  `contributes(c / (1 − c), <fact_term>, <fact_observed_state>)`
  clause. The extractor-LR enters the same aggregation as the
  domain-level LRs.

The `ε` cutoff is the calibration knob: deployments with
well-calibrated extractors can lower it; deployments with overconfident
extractors raise it. Default `ε = 0.05` mirrors common clinical
"definite" / "probable" / "possible" buckets.

**Backwards compatibility**: existing IR documents that set
`confidence = 1.0` continue to lower to `Probability::Certain`
unchanged. The grammar change is purely additive.

## Reinterpreting `NodeKind::Uncertainty`

Today `NodeKind::Uncertainty` is excluded from lowering (see
`adjudication-connector::lower_to_kb`, where the variant is a no-op).
ADJ14 makes it productive:

> An `Uncertainty` node represents a partially-observed atom whose
> presence the framework should weight by the extractor's stated
> confidence.

**Lowering rule for an `Uncertainty` node with `term: t, confidence:
c`**:

- Produce a single `ContributionClause { conclusion: t,
  evidence_term: <extractor_evidence>(t), logit_delta: log(c / (1 −
  c)) }` and add `<extractor_evidence>(t)` to the KB as an observed
  fact.
- Effectively: the extractor's observation that `t` *might be*
  present provides one log-odds unit of `log(c / (1 − c))` evidence
  for `t` actually being true.

This is the formal unification: an `Uncertainty` node and a `Fact`
node with `confidence < 1` lower to the *same* shape — an
extractor-level LR contribution. The distinction in the IR remains
useful for audit (which spans the model flagged as uncertain) and
for ADJ06 escalation (which uncertainties to ask about), but the
inference math is uniform.

## The inference algorithm (LP19e)

Pseudocode for evaluating `P(c | E)` under LR aggregation:

```python
def lr_aggregate(conclusion_term: Term, kb: KnowledgeBase) -> tuple[float, ProofDAG]:
    # 1. Find the prior; require exactly one
    priors = kb.priors_for(conclusion_term)
    if len(priors) == 0:
        return 0.5, empty_dag  # uniform fallback; warned in trail
    if len(priors) > 1:
        raise ConflictingPriors(conclusion_term)
    prior_logit = priors[0].prior_logit

    # 2. Collect every observed evidence atom that contributes to c
    contribs = []
    for clause in kb.contributions_for(conclusion_term):
        if kb.is_observed(clause.evidence_term):
            contribs.append(clause)

    # 3. Apply each contribution to the running logit
    logit = prior_logit
    for clause in contribs:
        logit += clause.logit_delta

    # 4. Apply joint contributions whose evidence set is fully observed
    joints = []
    for j in kb.joint_contributions_for(conclusion_term):
        if all(kb.is_observed(e) for e in j.evidence_set):
            logit += j.joint_logit_delta
            joints.append(j)

    # 5. Convert logit → probability and build the proof DAG
    posterior = sigmoid(logit)
    dag = build_lr_proof_dag(conclusion_term, prior_logit, contribs, joints, posterior)
    return posterior, dag
```

**Cost**: linear in the number of contributors. No 2ⁿ world
enumeration. This is the asymptotic win over ProbLog WMC for the
dominant probabilistic-adjudication shape; WMC remains available for
the cases that genuinely need it.

## The new proof step variant

`logic_engine::DerivationOrigin` grows a new variant:

```rust
pub enum DerivationOrigin {
    FromFact(FactId),
    FromRule(RuleId),
    /// LR contribution: this step applied `log(LR)` to the running
    /// log-odds of the conclusion. `clause_id` cites the originating
    /// `contributes(...)` rule; `evidence_fact_ids` cites the
    /// observed evidence that made this contribution active.
    FromContribution {
        clause_id: RuleId,
        evidence_fact_ids: Vec<FactId>,
        logit_delta: f64,
    },
    /// Joint LR contribution from a `contributes_jointly` clause.
    FromJointContribution {
        clause_id: RuleId,
        evidence_fact_ids: Vec<FactId>,
        joint_logit_delta: f64,
    },
    /// Prior establishment — appears once per LR-aggregation proof.
    FromPrior {
        clause_id: RuleId,
        prior_logit: f64,
    },
}
```

A complete LR-aggregation `Proof` looks like:

```text
Proof {
  bindings: {},
  steps: [
    ProofStep { goal: stomach_bug, origin: FromPrior { clause_id: R1, prior_logit: -2.20 } },
    ProofStep { goal: stomach_bug, origin: FromContribution { clause_id: R2, evidence_fact_ids: [F1], logit_delta: 1.386 } },
    ProofStep { goal: stomach_bug, origin: FromContribution { clause_id: R3, evidence_fact_ids: [F2], logit_delta: 1.099 } },
  ],
  via_facts: [F1, F2],
  via_rules: [R1, R2, R3],
  posterior_logit: 0.285,         // new field on Proof in LP19e
  posterior_probability: 0.571,   // new field
}
```

Two new fields are appended to `Proof` (additive, non-breaking):
`posterior_logit` and `posterior_probability`. They're `Option<f64>`
in the wire format so deterministic / WMC-only proofs leave them as
`None`.

## Worked example — differential diagnosis

Source (paraphrased medical chart):
```
Patient: diarrhea, vomiting, mild fever, no known drug allergy.
```

The framework extracts an IR with three `Fact` nodes for the observed
symptoms (all `confidence = 0.95`, lowered to `Certain`), one denied
`Fact` for the allergy, and one `Query` for `safe_to_discharge`.
The rulebook (compiled per ADJ09) contributes the relevant priors
and LRs:

| Rule node | Subtype                                         | Meaning                                                                  |
|-----------|-------------------------------------------------|--------------------------------------------------------------------------|
| R1        | `prior(0.10, stomach_bug)`                      | 10% base rate of stomach bug in the population reviewed                  |
| R2        | `contributes(4.0, diarrhea, stomach_bug)`       | Diarrhea has LR+ ≈ 4 for stomach bug                                     |
| R3        | `contributes(3.0, vomiting, stomach_bug)`       | Vomiting has LR+ ≈ 3                                                     |
| R4        | `contributes(1.5, mild_fever, stomach_bug)`     | Mild fever has weak LR+ ≈ 1.5                                            |
| R5        | `contributes(0.7, drug_allergy, stomach_bug)`   | Drug allergy *lowers* the prior on stomach bug (LR− = 0.7)               |
| R6        | `contributes_jointly(1.8, [diarrhea, vomiting], stomach_bug)` | When both are present, an extra synergy factor of 1.8                    |

Inference:

```text
λ₀ = log(0.10 / 0.90)              = −2.197    (prior logit)
   + log(4.0)                      = +1.386    (R2: diarrhea observed)
   + log(3.0)                      = +1.099    (R3: vomiting observed)
   + log(1.5)                      = +0.405    (R4: mild fever observed)
   + 0                             =  0.000    (R5: drug_allergy denied, not contributed)
   + log(1.8)                      = +0.588    (R6: joint synergy, both observed)
   ──────────────────────────────────────
   λ(stomach_bug | observed)       = +1.281
   P(stomach_bug | observed)       =  0.783
```

The human-readable derivation (ADJ16) renders this as:

> **P(stomach_bug | observed) = 78.3%.** Derived from a prior of 10%
> (R1) and the following observed contributions:
> - Diarrhea (F1, bytes 9..17): LR+ 4.0 → +1.39 log-odds (R2)
> - Vomiting (F2, bytes 19..27): LR+ 3.0 → +1.10 log-odds (R3)
> - Mild fever (F3, bytes 29..39): LR+ 1.5 → +0.41 log-odds (R4)
> - Joint synergy of diarrhea + vomiting: ×1.8 → +0.59 log-odds (R6)
>
> The denied drug allergy (F4, bytes 41..63) would have contributed
> LR− 0.7 (R5) if present; since it is denied, no contribution is
> applied.

Every clause id resolves to an IR node id; every IR node id resolves
to source spans; every span resolves to source bytes. The audit
trail makes this resolvable; the rendering surfaces it.

## Compatibility with existing ProbLog primitives

ADJ14 does **not** remove the existing `definitional` / `probabilistic`
/ `constraint` / `default` Rule subtypes. They remain in the
connector and in `logic-engine`. The two inference paths coexist:

- **WMC path** (`SearchMode::EnumerateAll`): possible-worlds
  enumeration over Bernoulli-distributed clauses. Used for
  ProbLog-style joint conjunctive rules and for queries that mix
  conjunctive deterministic rules with probabilistic indicators.
- **LR-aggregation path** (`SearchMode::LRAggregate`, new in LP19e):
  log-odds composition over `prior` + `contributes` clauses. Used
  when the query's conclusion is the target of one or more
  `contributes` clauses.

`SearchMode::AutoDetect` is extended:
1. If every clause touched by the query is `Certain` and no
   `contributes` clause names the conclusion: use `FindFirst`.
2. Else if at least one `contributes` clause names the conclusion:
   use `LRAggregate`.
3. Else: use `EnumerateAll` + WMC.

A single KB may contain both shapes — e.g., a clinical KB where the
diagnostic step is LR-aggregated but the discharge decision is a
deterministic rule over the diagnosis. The connector raises
`MixedShapeOnSameConclusion` if both a `contributes(*, *, c)` and a
`probabilistic(p, c, …)` are present for the same `c`, because the
math then becomes ambiguous and the deployment must choose.

## Audit trail implications (preview of ADJ15)

The `EngineArtifacts` struct (ADJ07) grows three additive fields:

```rust
pub struct EngineArtifacts {
    // …existing fields…
    pub independence_assumption_used: bool,            // new
    pub lr_contributions: Vec<LrContributionRecord>,    // new
    pub prior_record: Option<PriorRecord>,              // new
}
```

Every `contributes` step that fired during inference is recorded
once in `lr_contributions` with its clause id, the IR node id it
lowered from, the observed evidence facts, the LR value, and the
log-odds delta it added. A reviewer following the trail sees the
prior, every contribution, and the running logit — the entire
inference, by data, not commentary.

## Calibration — what's in scope, what's not

**In scope**: every `contributes` clause has an explicit numerical
LR. Where that number comes from (literature meta-analyses,
domain-expert elicitation, learned coefficients) is the rulebook
author's problem; ADJ14 surfaces and applies the LR but does not
claim to derive it.

**Out of scope**: learning LRs from labelled outcome data, drift
detection over time, posterior re-calibration when the population's
base rate changes. These are real engineering tasks but they
belong in ADJ09 (rule compilation) or a future "rule-calibration"
sub-spec.

**Important consequence**: the framework's verdicts are
*conditional on the rulebook's calibration being correct*. A
miscalibrated rulebook produces miscalibrated verdicts. The audit
trail makes this auditable — every LR comes from a cited rule node,
which cites a rulebook source span — but it does not make it
*correct*. Calibration remains a human responsibility.

## Conditional independence — the named assumption

LR aggregation under `λ = λ₀ + Σ log LRᵢ` is exactly correct when
the contributing evidence is **conditionally independent given the
conclusion**. Real-world evidence rarely satisfies this exactly:
diarrhea and vomiting are correlated even within stomach-bug cases.
The framework's responses:

1. **`contributes_jointly` is the explicit escape hatch.** A
   modeler who knows two findings are correlated declares it; the
   joint clause absorbs the correlation as an extra LR term.
2. **The independence assumption is recorded in the audit trail**
   (`engine_artifacts.independence_assumption_used: true`) whenever
   any LR aggregation produced the answer.
3. **The renderer surfaces the assumption** in the derivation prose:
   "assuming conditional independence of the listed contributions"
   appears verbatim in the rendered output (ADJ16).

These three are not a cure for the independence assumption's
limitations. They are an honest framing: the framework gives a
defensible verdict under the stated assumption, the audit trail
records the assumption, and the modeler is responsible for whether
the assumption is appropriate for the case at hand. This is what
Bayesian forensic statistics has always done; the framework
inherits the tradition.

## Open questions

1. **Calibration sensitivity.** How sensitive is `P(c | E)` to small
   errors in LR values? `contributes_jointly` partially absorbs
   correlation; what fraction of clinical realism is recoverable
   without it? Empirical follow-up; ADJ18 (sensitivity / VOI) is
   the natural home.
2. **Extractor confidence calibration in practice.** The default
   `ε = 0.05` cutoff and the `c / (1 − c)` mapping are crude. A
   future ADJ14b should benchmark actual LLM extractor confidence
   distributions on labelled corpora and propose a calibrated
   transform.
3. **Negative joint interactions.** `contributes_jointly` covers
   "evidence A and B together raise / lower more than the product
   of individual LRs." Does the framework need an analogous
   "evidence A and B together cancel each other out" primitive?
   Probably representable as `contributes_jointly(small_lr, [a, b],
   c)`; flagged here for follow-up.
4. **Streaming inference.** When new evidence arrives mid-pipeline
   (e.g., a clarification dialogue yields a new observed fact),
   recomputing from scratch is trivial. But does the framework
   want an incremental update API (`update_logit(conclusion,
   new_evidence)`)? Probably yes, as a performance optimization in
   LP19e v0.2.

## Limitations

1. **Per-conclusion `prior` requirement.** Conclusions without a
   declared prior default to 0.5 (uniform), which is rarely the
   right choice. The rulebook author has to put work into priors;
   the framework will not invent them.
2. **The independence assumption is the assumption.** Joint
   interaction terms ameliorate but do not eliminate it. Verdicts
   under LR aggregation are defensible *under the stated
   assumption*; the audit trail records the assumption; the modeler
   is on the hook for whether it's appropriate.
3. **Mixed-shape KBs require disambiguation.** A KB cannot
   simultaneously declare `contributes(*, *, c)` and
   `probabilistic(p, c, …)` for the same `c`. The connector
   rejects the combination; the deployment must pick a shape per
   conclusion.

## Status

Draft. The grammar is sufficient to implement; the engine work
lives in `LP19e` (to be drafted as a sibling sub-spec to LP19c and
LP19d).
The connector v2 (ADJ11 v2) will recognise the two new rule
subtypes and emit the new clause variants; this is purely additive
to the existing ADJ11.

## Where to read next

- [ADJ01](ADJ01-adjudication-ir-grammar.md) — the IR grammar this
  spec extends.
- [ADJ11](ADJ11-problog-connector.md) — the existing connector
  this spec grows a v2 of.
- [LP19](LP19-probabilistic-logic-core.md) — the engine foundation
  this spec depends on. LP19e will be the LR-aggregation sub-spec
  that complements LP19c (conditional probability via WMC ratio)
  and LP19d (approximate inference via Monte Carlo).
- [ADJ15](ADJ15-lowering-map-and-proof-dag.md) — the audit-trail
  integration that surfaces this spec's outputs.
- [ADJ16](ADJ16-derivation-rendering.md) — the human-readable
  prose renderer that turns LR-aggregation proofs into derivations
  a doctor / lawyer / analyst can defend.

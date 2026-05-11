# LP19c — Conditional Probability with Evidence: P(query | evidence)

## Overview

[`LP19`](LP19-probabilistic-logic-core.md) defines the probabilistic
engine and the marginal probability `P(query)`. This sub-spec adds
**conditional probability** — `P(query | evidence)` — which is the
shape every diagnostic and decision-support question actually takes.

> *"What's the probability of pneumonia?"* → marginal P(pneumonia)
>
> *"What's the probability of pneumonia **given** cough, fever, and
> abnormal chest X-ray?"* → conditional P(pneumonia | observations)

Medicine, finance, legal triage — every domain the framework targets
queries conditionally. This spec specifies the evidence-declaration
syntax, the ratio-of-WMCs computation, and the interaction with the
proof DAG and audit trail.

## Why a Separate Sub-Spec

Conditional probability is mechanical given the WMC backend — it is
the ratio of two WMC calls. But the *interface* deserves explicit
specification:

- **What does it mean to "assert evidence"** in a logic program? The
  natural reading is: condition every world on the evidence being true,
  then compute the query probability.
- **How is the evidence's source-span provenance recorded** in the
  proof DAG and audit trail? This must compose with ADJ06 (which feeds
  the IR's Facts to the engine as evidence) and ADJ07 (which records
  every clause's provenance).
- **What about contradictory evidence** (the evidence is impossible
  under the current model)? The engine must report this distinctly
  from "probability zero" — a model failure, not a query answer.

## Layer Position

```
   LP19  probabilistic logic core      ← marginal P(query), WMC backend
        │
        ▼
   LP19c conditional probability       ← this spec
        │
        ▼
   ADJ11 LP19 connector
        │
        ▼
   ADJ06 clarification (consumes evidence-related clarifications)
```

LP19c is purely additive to LP19. The marginal-probability API (`search`
in `SearchMode::AutoDetect`) continues to work unchanged. A new
`search_conditional(query, evidence, kb, mode)` API joins the
top-level surface.

## The Evidence Surface

Evidence is a set of `(Term, Boolean)` assertions:

```text
Evidence := {
    observations: [Observation],
}

Observation := {
    term:    Term,                        -- e.g., cough(patient)
    value:   true | false,                -- true: observed; false: ruled out
    source:  ObservationSource,
}

ObservationSource :=
    Direct         -- the user supplied this evidence as a Fact
  | Test(name)     -- the observation came from a named test result
  | Imputed        -- the engine inferred this from another observation
                      (rare; recorded so the trail is honest)
```

Each observation is a Boolean claim about a term's truth in the
*current world*, not its probability. The engine conditions on every
observation being satisfied.

## Semantics

Given query term `Q`, evidence `E`, and knowledge base `KB`:

```text
P(Q | E) = P(Q ∧ E) / P(E)

where:
    P(Q ∧ E) = WMC(φ_{Q ∧ E})    -- WMC over the formula that
                                     captures both Q's proofs AND
                                     every observation being true
    P(E)      = WMC(φ_E)         -- WMC over the formula that
                                     captures only the evidence
```

Two WMC calls. The engine's existing `enumerate_all` + WMC backend is
sufficient; the conditional layer is a wrapper that prepares the
appropriate formulas and divides.

Concretely:

1. **Compile evidence into temporary KB clauses.** Each
   `Observation { term, value: true }` is treated as if the term is
   provable in every relevant world. `value: false` is treated as if
   the term is *not* provable.
2. **Construct `φ_E`** as the conjunction of the evidence-clauses'
   Boolean indicators.
3. **Construct `φ_{Q ∧ E}`** by augmenting Q's proof DAG with the
   evidence conjunction.
4. **Compute both WMCs** using the standard backend.
5. **Return `P(Q ∧ E) / P(E)`**.

The standard arithmetic-precision caveats (LP19b) apply.

## Worked Example — Medical Diagnosis

From the discussion threaded through ADJ00 / ADJ11:

```prolog
% Priors
0.001 :: pneumonia.
0.05  :: viral_uri.

% Conditional symptom distributions
0.85 :: cough :- pneumonia.
0.10 :: cough :- \+ pneumonia, viral_uri.
0.05 :: cough :- \+ pneumonia, \+ viral_uri.

0.70 :: fever :- pneumonia.
0.30 :: fever :- \+ pneumonia, viral_uri.
0.02 :: fever :- \+ pneumonia, \+ viral_uri.

0.80 :: abnormal_cxr :- pneumonia.
0.01 :: abnormal_cxr :- \+ pneumonia.
```

The diagnostic query:

```text
search_conditional(
    query    = pneumonia,
    evidence = [Observation(cough,        true),
                Observation(fever,        true),
                Observation(abnormal_cxr, true)],
    kb       = above_kb,
    mode     = EnumerateAll,
)
```

Returns `P(pneumonia | cough ∧ fever ∧ abnormal_cxr)` — the probability
that the patient has pneumonia *given* these observations.

The same kb can be queried for the alternative diagnosis:

```text
search_conditional(
    query    = viral_uri,
    evidence = [...],
    kb       = same kb,
    mode     = EnumerateAll,
)
```

Returns `P(viral_uri | observations)`. The two probabilities should
not sum to 1 in general (they are not exhaustive — there may be other
diagnoses, or none of the above). Their *relative* size, plus the
gap from 1, is the differential diagnosis.

## Provenance and the Audit Trail

Every observation in the evidence set is itself an IR node (typically
a Fact, sometimes an Uncertainty after clarification). Its source span
is therefore already in the IR document. The evidence's contribution to
the conditional-WMC formula references this node id; the proof DAG's
audit trail records both the query's proofs *and* the evidence
conjunction.

In the worked-example terms above, the audit trail says:

```text
P(pneumonia | observations) = 0.74

Proof DAG:
  query proof 1: pneumonia (R1 prior)
    ──→ used clauses: R1 (prior), R2 (cough|pneumonia),
                       R3 (fever|pneumonia), R4 (cxr|pneumonia)
    ──→ evidence conjunction:
            Fact F1 (cough)         ← source span (note, 12, 18)
            Fact F2 (fever)         ← source span (note, 19, 24)
            Fact F3 (abnormal_cxr)  ← source span (radiology, 0, 35)

P(viral_uri | observations) = 0.18
P(neither)                   = 0.08
```

A reviewer follows any byte of the answer back to source bytes of the
clinical note. Same audit-trail discipline as the marginal case,
extended with evidence provenance.

## Contradictory Evidence

If the evidence asserts something the KB rules out (e.g., `value:
true` on a term that has no proof in any world), then `P(E) = 0` and
the conditional probability is undefined. The engine reports this as:

```text
ConditionalResult :=
    Probability(real in [0, 1])
  | ContradictoryEvidence {
        evidence_with_zero_mass: [Observation],
        proof_dag_for_evidence:   ProofDAG,
    }
```

Contradictory evidence is **not the same as a probability of zero
posterior**. It is a model failure — the KB cannot account for the
observation. The framework surfaces this to clarification (ADJ06) with
a specific question:

> "The observation '\<obs\>' is incompatible with the current rules.
>  Either the observation is wrong, or the rules need to be revised.
>  Which?"

This is a powerful diagnostic surface: it lets a clinician realize
their rule corpus is missing a case that real-world evidence
indicates.

## Soft vs. Hard Evidence

Some observations are themselves uncertain (the test had imperfect
sensitivity; the lab result has measurement error). The current spec
treats observations as **hard** — Boolean assertions about a term's
truth in the current world. **Soft** evidence (a probability
distribution over the observation's truth) is a richer extension.

Soft evidence is supported by replacing the Boolean indicator with a
fractional indicator: instead of `evidence(cough, true)`, the user
writes `evidence(cough, 0.95)` meaning "the observation 'cough' is
true with probability 0.95 (e.g., 95% specific test)."

The WMC computation generalizes: the evidence's contribution to `φ_E`
is `p · indicator + (1 - p) · ¬indicator` rather than just
`indicator`. Implementation is purely additive on top of hard evidence.

Soft evidence is `LP19c-extension`, in scope for the second wave.

## Decision-Theoretic Use

The conditional-probability API supports natural diagnostic and
decision queries:

```text
// Differential diagnosis ranking
P_pneumonia = search_conditional(pneumonia, observations, kb, EnumerateAll)
P_viral_uri = search_conditional(viral_uri, observations, kb, EnumerateAll)
ranked = [pneumonia: P_pneumonia, viral_uri: P_viral_uri, ...]

// Test value: would ordering X change the diagnosis?
P_before = search_conditional(pneumonia, current_observations, kb, EnumerateAll)
P_with_X_pos = search_conditional(pneumonia,
                                   current_observations + Observation(X, true),
                                   kb, EnumerateAll)
P_with_X_neg = search_conditional(pneumonia,
                                   current_observations + Observation(X, false),
                                   kb, EnumerateAll)
expected_info_gain = some_function(P_before, P_with_X_pos, P_with_X_neg)
```

The framework does **not** implement the `expected_info_gain`
function — that is decision-theoretic computation specific to a
domain. The conditional-probability primitive is what enables it.

## API Sketch

```rust
pub struct Observation {
    pub term: Term,
    pub value: bool,
    pub source: ObservationSource,
}

pub enum ObservationSource {
    Direct,
    Test(String),
    Imputed,
}

pub enum ConditionalResult {
    Probability(f64),
    ContradictoryEvidence {
        evidence_with_zero_mass: Vec<Observation>,
        proof_dag_for_evidence: ProofDAG,
    },
}

pub fn search_conditional(
    query: &Term,
    evidence: &[Observation],
    kb: &KnowledgeBase,
    mode: SearchMode,
) -> ConditionalResult;
```

`mode` is the same as the marginal `search` — `AutoDetect` selects
`FindFirst` only when the KB is all-Certain *and* the evidence is also
fully resolved (which would be a degenerate case for conditional
queries; typically `EnumerateAll` is selected).

## Open Questions

1. **Negation in evidence.** `evidence(cough, false)` is "the patient
   does not have cough." Under negation-as-failure, this is the
   absence of a proof for `cough`. The current spec adopts this
   reading; future work may explore explicit classical negation.
2. **Joint queries.** `P(pneumonia ∧ ¬viral_uri | observations)` —
   composing query terms. Currently a single query term; composition
   is straightforward via auxiliary rules but the surface API may want
   support.
3. **Sequential evidence.** Real diagnoses accumulate evidence over
   time. The framework currently treats evidence as a static set; an
   extension that supports `update(prior, new_observation)` for online
   diagnosis is `LP19c-online`.
4. **Sample-based inference for large evidence sets.** When the
   evidence is large (full clinical note, many observations), exact
   WMC may not scale. Monte Carlo conditional sampling is the obvious
   fallback; specified in `LP19d`.

## Limitations

1. **The independence assumptions in the KB matter even more for
   conditional queries.** Two observations modeled as independent that
   are actually dependent will produce miscalibrated posteriors. This
   is a modeling concern, not an engine concern.
2. **Floating-point precision.** When `P(E)` is very small (rare
   evidence pattern), the division `P(Q ∧ E) / P(E)` can amplify
   numerical error. LP19b's rational-arithmetic mode is the
   recommended fallback for high-stakes queries.
3. **Contradictory-evidence surface is honest but blunt.** A more
   refined version would suggest *which* observation is least
   compatible with the KB. Out of scope here.

## Status

Draft. Sufficient to implement on top of LP19's existing WMC backend.
Soft evidence (`LP19c-extension`) and online updates (`LP19c-online`)
are scheduled as follow-up sub-specs as deployment experience demands.

# ADJ11 — ADJ to LP19 Connector: Probabilistic Adjudication

## Overview

[`LP19`](LP19-probabilistic-logic-core.md) defines the probabilistic Logic
VM extension as a *uniform algebra* — deterministic Prolog is the
degenerate case of probabilistic reasoning. This spec is the thin
connector that bridges [`ADJ`](ADJ00-adjudication-framework.md) to
LP19: it specifies how the Adjudication IR's `Rule` subtypes lower to
LP19 clauses, how queries against the ADJ pipeline reach the LP19
engine, and how the engine's proof DAG flows back through the
audit trail.

Earlier drafts of this spec carried a complete probabilistic engine. The
LP19 architectural decision relocated that work to the Logic VM layer.
ADJ11 is now small and well-bounded — a connector spec, not an engine
spec.

## Layer Position

```
   ADJ00 framework               ← the adjudication architecture
        │
        ├── ADJ01 IR grammar     ← typed IR with Fact/Rule/etc.
        │
        ├── ADJ02..ADJ05 checker passes
        │
        ▼
   ADJ11 ProbLog connector       ← this spec
        │
        ▼
   LP19 probabilistic logic core ← the engine; this is where work lives
        │
        ▼
   LP00..LP18 logic core         ← terms, unification, base engine
```

ADJ11 is a *thin glue layer*. It says nothing about how probabilities are
computed; that is wholly LP19's job. It says only how ADJ's pipeline
delivers facts and rules to LP19 and how the engine's response is woven
back into ADJ's audit trail.

## What This Spec Defines

1. **Lowering rules** from each ADJ Rule subtype to an LP19 `Rule` value.
2. **Wrapping rules** for ADJ Fact, Query, and Uncertainty nodes when
   they reach the engine.
3. **The query / evidence protocol** — how the ADJ pipeline frames a
   diagnostic-shape question (`P(diagnosis | observed_facts)`) as an
   LP19 query.
4. **The proof-DAG bridge** — how LP19's returned proof DAG nodes link
   back into ADJ IR nodes for audit-trail purposes.

## Lowering ADJ Rules to LP19

The ADJ Rule subtypes (from `ADJ01`) map to LP19 as follows:

### Definitional

```text
ADJ:  Rule { term: definitional(head, body), polarity: Affirmed,
             modality: Present, source_spans: [...], metadata: { as_of } }

LP19: Rule {
          id:          fresh RuleId,
          head:        head,
          body:        body.map(BodyLiteral::Pos),
          probability: Certain,
      }
```

The `as_of` metadata is preserved on the LP19 Rule's metadata channel
(LP19 carries an opaque metadata bag on each clause for this purpose; the
field is added in `LP19b`).

### Constraint

A constraint has no head — it is a body that must hold. The lowering
introduces a synthetic head with a unique functor:

```text
ADJ:  Rule { term: constraint(body), polarity: Affirmed, ... }

LP19: Rule {
          id:          fresh RuleId,
          head:        compound("_constraint", [Atom("c_<n>")]),
          body:        body.map(BodyLiteral::Pos),
          probability: Certain,
      }
```

The adjudication asserts `_constraint(c_<n>)` as part of every query;
clause selection ensures the constraint is checked. The synthetic
functor `_constraint` is reserved.

### Default with Exceptions

A default rule "X applies unless an exception" lowers to a positively
stated rule with `Neg` body literals for each exception:

```text
ADJ:  Rule { term: default(head, body, exceptions), ... }

LP19: Rule {
          id:          fresh RuleId,
          head:        head,
          body:        body.map(BodyLiteral::Pos)
                       ++ exceptions.map(BodyLiteral::Neg),
          probability: Certain,
      }
```

Priority among multiple defaults is encoded by clause ordering in the
LP19 knowledge base: higher-priority rules are inserted first and the
engine's clause-selection order ensures they are tried first.

### Probabilistic

```text
ADJ:  Rule { term: probabilistic(p, head, body), ... }

LP19: Rule {
          id:          fresh RuleId,
          head:        head,
          body:        body.map(BodyLiteral::Pos),
          probability: Value(p),
      }
```

This is the direct case. The `p` in the ADJ IR becomes the
`Value(p)` in the LP19 Rule. The engine, on receiving any rule with
`Value(p)`, switches its `AutoDetect` to `EnumerateAll` and the
weighted-model-counting backend computes the query's probability.

## Wrapping ADJ Facts, Queries, and Uncertainties

ADJ `Fact` nodes lower to LP19 `Fact` values:

```text
ADJ:  Fact { term: t, polarity: Affirmed, ... }
LP19: Fact { id: fresh, term: t, probability: Certain }

ADJ:  Fact { term: t, polarity: Denied, ... }
LP19: Rule { id: fresh, head: t, body: [BodyLiteral::Neg(t)],
             probability: Certain }
```

A `Denied` fact becomes a *rule that succeeds when `t` cannot be proved*,
which captures the "denied" semantics under negation-as-failure. This
is the only place the polarity-to-clause translation departs from a
purely additive mapping, and it is justified by the fact that LP19's
semantics is well-founded over the negation-as-failure reading.

ADJ `Query` nodes lower directly to LP19 queries:

```text
ADJ:  Query { term: t, ... }
LP19: query t against the assembled knowledge base
```

ADJ `Uncertainty` nodes lower to LP19 **evidence** declarations (per
`LP19c` once that sub-spec lands) when the engine is asked to compute
conditional probabilities `P(diagnosis | uncertain observations)`.
Pending `LP19c`, uncertainty is recorded in the audit trail and routed
to clarification (`ADJ06`).

## Query / Evidence Protocol

The diagnostic shape of an adjudication is:

> Given these observed facts (the ADJ Facts), these declared
> uncertainties (the ADJ Uncertainties), and these rules (compiled from
> the rulebook into ADJ Rules), what is the probability of each
> candidate answer to the Query?

In LP19 terms:

```text
P(query_term | evidence) = WMC(φ_query_and_evidence) / WMC(φ_evidence)

where:
    φ_evidence  = conjunction of indicators for every Fact and
                  Uncertainty asserted as observation
    φ_query     = the Boolean formula from the proof DAG of the query
    φ_query_and_evidence = φ_query ∧ φ_evidence
```

This is the conditional-probability protocol described in `LP19c`. The
adjudication pipeline assembles the knowledge base (Facts +
Uncertainties as evidence; Rules as the program), submits the Query,
receives the proof DAG and the WMC, and surfaces the result.

For deterministic adjudications (the TSA case, license compatibility),
the entire pipeline runs LP19's `FindFirst` short-circuit and looks
identical to classical Prolog. The user does not have to *opt in* to
the deterministic path; LP19's `AutoDetect` selects it automatically
when no clause has `probability != Certain`.

## Proof DAG to Audit Trail

LP19's proof DAG nodes carry `via_facts` and `via_rules` lists naming
the LP19 clauses each derivation step used. The ADJ audit trail
(specified in `ADJ07`) records, for each LP19 clause id, the ADJ IR
node id it was lowered from.

The proof DAG therefore reads, end-to-end:

```text
LP19 proof step  ──cites──▶  LP19 clause id  ──maps to──▶  ADJ IR node id
                                                                │
                                                                ▼
                                                       ADJ IR node's source spans
                                                                │
                                                                ▼
                                                       original document byte ranges
```

Every step is materialized data, not post-hoc reconstruction. A reviewer
following the proof DAG arrives at the original document character by
character.

## Worked Example: Differential Diagnosis

Following the medical mapping from prior discussion:

ADJ IR (simplified; assume coverage, polarity, modality checks passed):

```text
F1  Fact { term: cough(patient),          polarity: Affirmed, ... }
F2  Fact { term: fever(patient),          polarity: Affirmed, ... }
F3  Fact { term: abnormal_cxr(patient),   polarity: Affirmed, ... }

R1  Rule { term: probabilistic(0.001, pneumonia(P), [adult(P)]), ... }
R2  Rule { term: probabilistic(0.85,  cough(P),     [pneumonia(P)]), ... }
R3  Rule { term: probabilistic(0.70,  fever(P),     [pneumonia(P)]), ... }
R4  Rule { term: probabilistic(0.80,  abnormal_cxr(P), [pneumonia(P)]), ... }

Q   Query { term: pneumonia(patient), ... }
```

Lowered to LP19:

```text
KB.facts = [
    Fact { id: 1, term: cough(patient),        probability: Certain },
    Fact { id: 2, term: fever(patient),        probability: Certain },
    Fact { id: 3, term: abnormal_cxr(patient), probability: Certain },
    Fact { id: 4, term: adult(patient),        probability: Certain }, // assumed
]
KB.rules = [
    Rule { id: 1, head: pneumonia(P),     body: [Pos(adult(P))],
           probability: Value(0.001) },
    Rule { id: 2, head: cough(P),         body: [Pos(pneumonia(P))],
           probability: Value(0.85) },
    ... etc.
]

Query: pneumonia(patient)
Evidence (implicit): cough(patient), fever(patient), abnormal_cxr(patient)
```

LP19 detects at least one rule with `Value(p)` and switches to
`EnumerateAll`. The proof DAG is constructed, the Boolean formula is
emitted, and weighted model counting returns `P(pneumonia(patient) |
evidence)`. ADJ receives the number and the DAG, attaches the audit
trail, and emits the final answer:

```text
{
  "answer":      "pneumonia",
  "probability": 0.74,
  "proof_dag":   <LP19 DAG annotated with ADJ source spans>,
  "audit_trail": <document spans → IR nodes → LP19 clauses → DAG path>
}
```

A reviewer can replay every step.

## Independence and Calibration — Out of Scope

The independence assumption (`LP19` Limitation 2) and calibration
question (`LP19` Limitation 3) apply unchanged here. ADJ11 does not
"solve" calibration; it surfaces whatever probabilities the ADJ rule
pipeline produced. Calibration is the modeler's responsibility, and the
ADJ09 rule-compilation pipeline is the place where literature-extracted
likelihood ratios would land if/when that work is undertaken.

## Open Questions

1. **Mixed-mode knowledge bases.** An adjudication may combine
   deterministic constraints (e.g., TSA rules) with probabilistic
   diagnostic rules (e.g., differential diagnosis). LP19 handles this
   correctly per its semantics, but the *clarification UX* in such
   cases — "this constraint is hard, this rule is probabilistic" —
   needs explicit user-facing design (deferred to `ADJ06`).
2. **Numerical stability.** Very small probabilities (`p = 1e-9` for
   rare-disease priors) interact poorly with floating-point arithmetic
   in WMC. `LP19b` (rational arithmetic) addresses this; ADJ11
   inherits.
3. **Decision-theoretic outputs.** "Should we order this test?" is a
   decision question, not a probability question. It composes
   probabilities with utilities. Out of scope for ADJ11; a future
   sub-spec might extend the connector.

## Limitations

1. ADJ11 is *only* a connector. Bugs in the engine show up as bugs in
   ADJ11 outputs; fix them in `LP19`.
2. The `_constraint` synthetic functor introduces a small reserved
   namespace inside LP19 knowledge bases. Documents that contain a
   user-declared functor of the same name are rejected at lowering
   time.
3. Polarity-to-clause translation for `Denied` facts uses
   negation-as-failure, which is correct only for stratified programs
   (per LP19). Non-stratified ADJ IRs (rare; possible only if the rule
   pipeline produces them) are rejected at lowering time with an
   explanatory error.

## Status

Draft. The lowering rules are sufficient to implement; the connector
crate (`code/packages/rust/adjudication-connector`, to be created) will
exercise this spec as soon as both LP19's full inference path and ADJ's
extraction path have working Rust implementations.

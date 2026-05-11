# ADJ03 — Polarity and Modality Checker: Propagation Consistency

> **Revision v2 (2026-05-11): propagation consistency.** v1 of this
> spec described a NegEx/ConText-style trigger-detection check that
> baked English clinical idiom into the framework core. This revision
> replaces the trigger taxonomy with a **structural propagation
> consistency check** over the hierarchical IR from
> [`ADJ01` v2](ADJ01-adjudication-ir-grammar.md). The LLM that
> produces the IR is now responsible for deciding *what* the polarity
> and modality of each node are (in any language). The framework's
> only job is to verify that the polarity and modality values the
> LLM declared on each node **propagate consistently** through the
> decomposition tree.

## Overview

The propagation invariant in v2 is:

> A leaf's *effective* polarity (and modality) is the polarity (and
> modality) declared on the nearest ancestor — or, if no ancestor
> declares a value, the leaf's own declaration. The leaf's *declared*
> polarity must match this effective value, or must be `Inherit`.

In short: a child cannot silently contradict its ancestor's polarity
or modality. If the ancestor says "the patient denies the following:"
and a leaf wants to assert (rather than deny) one of those items, the
leaf must say so *explicitly* — its declared polarity overrides the
inherited one, the override is visible in the audit trail, and the
inconsistency surfaces as a clarification opportunity rather than a
silent flip.

There is no trigger taxonomy. There is no scope detector. There is
no NegEx machinery. The LLM's hierarchical decomposition already
encodes the linguistic information; ADJ03 verifies the encoding is
self-consistent.

## What the Check Catches

| v1 (rule-based) caught | v2 (propagation) catches |
|---|---|
| English negation triggers misaligned with leaf polarity | A leaf whose declared polarity contradicts the ancestor's propagated polarity without explicit override semantics |
| Hedge/temporality/family-history English cues | Same failure classes, but driven by the LLM's hierarchical declarations rather than per-node lexical detection |
| RuledOut vs. Denied distinction enforced by trigger class | Same distinction preserved by ADJ01 v2's separate fields |

The class of failure caught is the same: silent flipping of polarity
or modality. What changes is the mechanism: instead of finding
English negation triggers in source spans, the framework checks that
the LLM's own ancestor-vs-leaf declarations are coherent.

## Layer Position

```
   ADJ01 v2 hierarchical IR
        │
        ▼
   ADJ02 v2 structural coverage
        │
        ▼
   ADJ03 v2 propagation consistency   ← this document
        │
        ▼
   ADJ04 round-trip entailment
        │
        ▼
   ADJ05 adversarial verifier
```

Propagation runs after coverage so it can assume the tree is
well-formed (every byte covered, parents above children). The
propagation check itself is independent of source text — it operates
on the tree structure and the polarity/modality fields only.

## Effective Polarity / Modality

`Polarity` and `Modality` carry an `Inherit` value in v2 (per
ADJ01 v2). The **effective** value at a node is computed:

```text
effective_polarity(node, tree) =
    if node.polarity != Inherit:
        node.polarity
    elif node.part_of is Some(parent_id):
        effective_polarity(tree.find(parent_id), tree)
    else:
        Affirmed                     # the framework's outermost default

effective_modality(node, tree) = ... (same shape; default Present)
```

`Inherit` says "use the ancestor's value." A non-`Inherit` value at a
node *overrides* the ancestor's, and the override is what the leaf
contributes to downstream reasoning.

## The Five Propagation Conditions

For an IR document to satisfy propagation consistency:

1. **Inherit-resolvable**: every node's `effective_polarity` (and
   `effective_modality`) resolves to a non-`Inherit` value within
   the document. (No `Inherit` chain reaches the root with the root
   itself declared `Inherit`.)
2. **Leaf-vs-ancestor consistency**: when a leaf's *declared*
   polarity is non-`Inherit` and *differs* from its ancestor's
   propagated value, the difference must be the leaf's intent (it
   IS the override). The framework flags such overrides for review
   but does not fail by default — they are warnings, not errors.
3. **Conflict-with-itself**: a leaf cannot declare two contradictory
   things. (Trivially enforced by the IR's well-formedness; one node
   has one polarity field.)
4. **RuledOut + Denied separation**: if a leaf's modality is
   `RuledOut`, its polarity must be `Affirmed`. The clinical/legal
   distinction is preserved as a hard rule.
5. **Discarded nodes**: skipped from the check. Their polarity and
   modality are formally `Affirmed` and `Present` per ADJ01.

The check produces *violations* for conditions 1, 3, 4 and *warnings*
for condition 2. Warnings are surfaced in the audit trail and may
trigger clarification per ADJ06 but do not gate the adjudication by
default. A deployment can configure warnings-as-errors if it wants
strict semantics.

## The Algorithm

```text
propagation_check(ir_doc) -> Result:
    violations = []
    warnings   = []
    by_id = index_nodes(ir_doc)

    # Pre-compute effective values for every node.
    eff_polarity = {}   # node_id -> Polarity (non-Inherit)
    eff_modality = {}

    for node in ir_doc.nodes:
        eff_polarity[node.id] = resolve(node, by_id, .polarity, Affirmed)
        eff_modality[node.id] = resolve(node, by_id, .modality, Present)
        if eff_polarity[node.id] == Inherit or eff_modality[node.id] == Inherit:
            violations.push(InheritChainUnresolved { node.id })

    # Walk every leaf; compare declared vs. effective; flag overrides.
    for node in ir_doc.nodes:
        if node.kind == TextRun: continue
        if node.kind == Discarded: continue

        if node.modality == RuledOut and node.polarity != Affirmed:
            violations.push(RuledOutMustBeAffirmed { node.id })

        if node.part_of is Some(parent_id):
            parent_p = eff_polarity[parent_id]
            parent_m = eff_modality[parent_id]
            if node.polarity != Inherit and node.polarity != parent_p:
                warnings.push(LeafOverridesAncestorPolarity {
                    node.id, declared: node.polarity, ancestor: parent_p
                })
            if node.modality != Inherit and node.modality != parent_m:
                warnings.push(LeafOverridesAncestorModality {
                    node.id, declared: node.modality, ancestor: parent_m
                })

    return Pass(warnings) if violations is empty else Fail(violations, warnings)
```

The walk is `O(N)` over IR nodes. Effective-value resolution is
`O(depth)` per node, `O(N × depth)` total, with memoisation reducing
to `O(N)`.

## Violation and Warning Types

```text
PropagationViolation :=
    InheritChainUnresolved { node_id }
  | RuledOutMustBeAffirmed { node_id }

PropagationWarning :=
    LeafOverridesAncestorPolarity { node_id, declared, ancestor }
  | LeafOverridesAncestorModality { node_id, declared, ancestor }
```

Violations gate the adjudication. Warnings are recorded in the audit
trail and may be promoted to violations via configuration. The
default policy is **warn-do-not-block** because legitimate overrides
exist in real documents (e.g., a paragraph that lists denied
symptoms but mentions one the patient does affirm: *"Denies chest
pain, fever, palpitations; admits shortness of breath."*).

## Clarification Generation

| Violation / warning | Clarification question shape |
|---|---|
| `InheritChainUnresolved` | Internal: re-prompt the LLM (the IR is malformed) |
| `RuledOutMustBeAffirmed` | Internal: re-prompt the LLM (ADJ01 hard rule) |
| `LeafOverridesAncestorPolarity` | *"In the context '<parent text>' (denied), this claim '<leaf text>' is marked affirmed. Is the patient affirming this one item despite denying the surrounding context, or should this be denied too?"* |
| `LeafOverridesAncestorModality` | Similar shape for modality overrides |

Override warnings are the genuinely interesting case. They surface
the cases where the LLM made a structural choice that may need human
review.

## RuledOut vs. Denied — Preserved

The clinical and legal distinction between *RuledOut* (clinician's
adjudication) and *Denied* (patient's claim) was emphasised in v1
and is preserved in v2 as a hard rule (`RuledOutMustBeAffirmed`).

The propagation mechanism doesn't blur this. A parent with `modality:
RuledOut` propagates RuledOut to descendants, who can override; a
parent with `polarity: Denied` propagates Denied. They never collapse
into each other.

## Comparison to v1

| Aspect | v1 (rule-based) | v2 (propagation) |
|---|---|---|
| Trigger taxonomy (NegEx, ConText) | Required, English-specific | Removed |
| Scope detection | Required | Removed |
| Languages supported | English (effectively) | Any language the LLM handles |
| Algorithm | Per-node scope analysis | Tree-shape propagation |
| Asymptotic complexity | `O(triggers × nodes × span_text_len)` | `O(N)` over IR nodes |
| Determinism | Yes (rule-based) | Yes (structural) |
| LLM calls at check time | Zero | Zero |
| Failure modes caught | Silent polarity flip via missed trigger | Silent polarity flip via inconsistent declaration |
| False-positive rate | Domain-dependent (high in non-clinical text) | Zero — by construction |
| RuledOut vs. Denied distinction | Enforced via trigger class | Enforced by ADJ01 v2 hard rule |

## Worked Example (Revised)

Continuing the TSA case from ADJ02 v2's worked example. The IR has a
parent `N2 TextRun` with `polarity = Denied` that wraps the *"I am not
bringing matches"* span; its child `F6` has `polarity = Inherit`.

Propagation:
- `eff_polarity[N2] = Denied` (declared).
- `eff_polarity[F6] = eff_polarity[N2] = Denied` (inherited).

`F6` is a Fact leaf with `polarity = Inherit`. The effective polarity
is Denied, which matches what the LLM intended (the patient is *not*
bringing matches). No override; no warning; the check passes.

Counterexample: suppose the LLM gets confused and emits `F6` with
`polarity = Affirmed` directly (declaring "yes, bringing matches")
while still being a child of `N2` (declared "not bringing"). The
propagation check flags this:

```text
LeafOverridesAncestorPolarity {
    node_id: F6,
    declared: Affirmed,
    ancestor: Denied (from N2),
}
```

Clarification: *"In the context 'I am not bringing matches' (denied),
this claim about matches is marked affirmed. Should it be denied?"*
The LLM (or user) confirms or corrects; the IR updates; the check
re-runs.

A *legitimate* override case: under a `Denied` parent listing
multiple items, one item is actually affirmed. Example:

```text
TextRun polarity=Denied   "Denies chest pain, fever, palpitations; admits shortness of breath."
  Fact chest_pain        polarity=Inherit     → effective Denied  ✓
  Fact fever             polarity=Inherit     → effective Denied  ✓
  Fact palpitations      polarity=Inherit     → effective Denied  ✓
  Fact sob               polarity=Affirmed    → override! warning
```

The warning lets a reviewer (or a downstream LLM call in ADJ04) read
the source span and verify that "admits shortness of breath" is a
genuine contrast within a denial list. Real medical prose does this
all the time; the framework surfaces it for review rather than
silently blocking.

## Open Questions

1. **Default behaviour for unresolved Inherit.** Currently if every
   ancestor up to the root declares `Inherit`, the effective value
   falls back to `Affirmed` (polarity) or `Present` (modality). This
   is the framework default, recorded explicitly in the audit trail
   so reviewers can challenge it. Alternative: require every IR
   document to have a non-`Inherit` value at every root. Either is
   defensible; the current default favours leniency.
2. **Promoting warnings to errors.** The default `warn-do-not-block`
   policy is permissive. A deployment that wants strict propagation
   can configure warnings-as-errors. The line between "real
   override" and "extraction mistake" is empirical — the deployment
   chooses based on its observed false-positive rate.
3. **Multi-language consistency.** The framework's check is
   language-agnostic, but the LLM's polarity/modality decisions are
   language-specific. A deployment in a low-resource language may
   see higher rates of `LeafOverridesAncestorPolarity` warnings as
   extraction quality drops. The audit trail's per-language metrics
   will show this; the framework itself has nothing to tune.
4. **What about cross-document polarity?** A clinical note may
   reference a prior note: *"As discussed before, denies the
   following:"*. The reference is in this document; the polarity
   propagates from the parent text run. Cross-document propagation
   (the prior note's polarity bleeding into this one) is out of
   scope. Each document's tree is independent.

## Limitations

1. **The check trusts the LLM's polarity declarations.** If the LLM
   wrongly emits `polarity: Affirmed` on a parent that should be
   `Denied`, propagation alone cannot detect it. ADJ04 (round-trip)
   and ADJ05 (adversarial) are the safety nets for *content*
   mistakes; ADJ03 only verifies *consistency*.
2. **Override warnings are inherently noisy.** A deployment that
   produces many overrides legitimately (medical prose with
   "denies X, Y; admits Z" structures) will see many warnings.
   This is by design — the warnings highlight cases worth a second
   look, not cases worth blocking.
3. **No language-specific shortcuts.** The v1 rule-based approach
   was very cheap on cases like "denies chest pain"; in v2 the cost
   is paid up front in extraction (the LLM must produce the
   hierarchical structure correctly). For high-volume English
   clinical use the v1 path *may* still be a useful optimisation,
   available as an opt-in domain accelerator — but it is no longer
   the default and no longer part of the framework core.

## Status

v2 draft. Replaces the v1 trigger-detection path. Implementation
depends on the `ADJ01` v2 grammar; the Rust crate
`adjudication-polarity-modality` is being rewritten in parallel
(the v1 trigger taxonomy is retired).

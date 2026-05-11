# ADJ03 — Polarity and Modality Checker: Catching the "Denies Chest Pain" Class

## Overview

This is the second of the four checker passes from [`ADJ00`](ADJ00-adjudication-framework.md).
After [`ADJ02`](ADJ02-coverage-checker.md) has confirmed that every
meaningful span of input is accounted for by some IR node, this pass
verifies that the *polarity* (Affirmed / Denied / Uncertain) and the
*modality* (Present / Past / Future / Hypothetical / FamilyHistory /
RuledOut / Conditional) of each node are consistent with the lexical
and syntactic evidence in the span it cites.

The failure mode this catches is the canonical one:

> Source span: *"patient denies chest pain"*
> IR node:    `Fact { term: chest_pain(patient), polarity: Affirmed }`
> Coverage:   passes — the entire span is covered.
> Polarity:   **fails** — "denies" is an in-scope negation trigger,
>             polarity should be Denied, not Affirmed.

Coverage was designed to be cheap and catch silent omission. Polarity is
designed to be cheap and catch silent flipping. Both run before any LLM
call at check time, both are deterministic, both produce a specific
clarification question on failure.

## Layer Position

```
   ADJ02 coverage              ← already ran, every meaningful span covered
        │
        ▼
   ADJ03 polarity/modality     ← this document
        │
        ▼
   ADJ04 round-trip entailment
```

ADJ03 is the rule-based scope detector in the lineage of **NegEx**
(Chapman et al. 2001) and **ConText** (Harkema et al. 2009). The
algorithm generalizes that lineage to a configurable trigger taxonomy
covering polarity *and* modality together, since the two interact
constantly in practice (e.g., "father had MI" is family-history modality
*and* affirmed polarity; "denies family history of cancer" is a denied
claim *about* family history).

## What This Pass Is Responsible For

For every IR node of kind `Fact`, `Query`, `Uncertainty`, or `Rule`, the
pass examines the union of the node's source spans and checks:

1. **Negation scope.** Are there negation triggers ("not", "denies",
   "without", "negative for", "ruled out", "no"...) whose syntactic
   scope covers the term's content words? If yes, the node's polarity
   must be `Denied`.
2. **Hedge / uncertainty triggers.** Are there hedge cues ("possibly",
   "questionable", "consistent with", "suggestive of", "differential
   includes")? If yes, the node should be `Uncertainty`, not `Fact`.
3. **Temporality triggers.** Are there past-tense cues, history markers,
   or explicit dates ("history of", "in 2019", "previously",
   "currently", "on admission")? Modality should align with whatever
   the cue indicates.
4. **Subject triggers.** Are there family-relation tokens ("father",
   "mother", "brother", "family history of") in scope? If yes, the
   subject of the predicate is **not** the patient, and modality should
   be `FamilyHistory`.
5. **Conditional triggers.** "If", "when", "in case of" — these mark a
   `Conditional` modality. The node's term may also need a structural
   `condition` argument (see open questions below).
6. **Rule-out triggers.** "Ruled out", "excluded by", "negative on CT"
   — these are *not* the same as negation. Modality must be `RuledOut`,
   and polarity remains `Affirmed` (we affirm that the diagnosis was
   considered and excluded).

Each check has the shape: *"if the span contains a trigger with scope
covering the term, then a specific (polarity, modality) field must take
a specific value."* The pass reports the first violation it finds per
node, with the trigger, its scope, and the required value.

## The Trigger Taxonomy

Triggers are organized by class and direction. Direction matters
because a trigger like "denies" applies to what *follows* it, while
"no" can apply in either direction depending on syntax. The taxonomy
draws directly from NegEx/ConText and is configurable for new domains.

```text
Trigger := {
    class:      TriggerClass,
    surface:    string,            -- the lexical form
    direction:  Forward | Backward | Bidirectional,
    scope:      ScopeRule,
}

TriggerClass :=
    Negation              -- "not", "no", "denies", "without"
  | Hedge                  -- "possibly", "questionable"
  | TemporalPast           -- "history of", "in 2019", "previously"
  | TemporalPresent        -- "currently", "today", "on admission"
  | TemporalFuture         -- "will", "anticipated", "planned"
  | Hypothetical           -- "if X then Y", "in case of"
  | FamilyHistory          -- "father", "mother", "family history of"
  | RuleOut                -- "ruled out", "excluded", "negative for"
  | Subject                -- "patient", "she", "he" (for entity attribution)

ScopeRule :=
    UntilSentenceEnd
  | UntilPunctuation([;., ])
  | UntilTermination(["but", "however", "except", "although"])
  | UntilTokenCount(n)
  | UntilCueOpposite             -- e.g., another negation cancels
```

A trigger has effective scope from its position until the first
matching `ScopeRule` boundary in the trigger's `direction`. Concretely,
"denies" with `Forward` direction and `UntilSentenceEnd` scope applies
to every token between itself and the next sentence-ending punctuation.

## The Algorithm

```text
check_polarity_modality(ir_doc, doc, taxonomy):
    for node in ir_doc.nodes:
        if node.kind not in [Fact, Query, Uncertainty, Rule]:
            continue
        span_text = doc.text(union(node.source_spans))
        triggers  = find_triggers(span_text, taxonomy)
        for trigger in triggers:
            scope = compute_scope(trigger, span_text, taxonomy)
            if not term_content_in_scope(node.term, span_text, scope):
                continue
            required = required_field(trigger, node)
            actual   = node_field(trigger.class, node)
            if actual != required:
                emit_violation(node, trigger, required, actual)
                break  // first violation per node is enough to fail
```

`term_content_in_scope` checks whether the *content words* of the
node's term — the functor name and ground-atom arguments — appear inside
the trigger's effective scope. This is a small surface-language check;
it operates on the source text, not on the abstract term.

`required_field` and `node_field` are small lookup tables encoding the
rule-out-cases-by-class summary above. For example, for a Negation
trigger:

```text
required_field(Negation, _) = (polarity: Denied)
node_field    (Negation, node) = (polarity: node.polarity)
```

If they disagree, the violation is reported. The exact API for
violations is in `ADJ06` (clarification), but the data shape is
straightforward:

```text
Violation := {
    node_id:    NodeId,
    trigger:    Trigger,
    span:       Span,                  -- where the trigger occurred
    required:   PolarityOrModality,
    actual:     PolarityOrModality,
    suggestion: string,                -- human-readable clarification cue
}
```

## Interaction with Coverage

ADJ02 has already guaranteed that every meaningful span is inside the
union of node source-spans. ADJ03 needs only the *intra-node* check
above. It does **not** need to inspect the global document; it can run
node by node, independently, in parallel.

One small global check remains: a trigger token (e.g., the word
"denies") that is inside a node's spans is itself a meaningful token by
the coverage taxonomy. Coverage will have accepted it as covered iff
the node it belongs to also accepted it. The polarity check now
re-examines whether the node *responded to* the trigger correctly.

## Triggers Whose Scope Crosses Node Boundaries

A negation trigger inside node A may have scope extending into the
text covered by node B. The framework's policy:

- Each node is checked against triggers in its **own** source spans
  only. Cross-node triggers are not the per-node check's concern.
- A separate **global cross-node negation check** runs after the
  per-node checks. It uses the document's full text plus the IR's span
  index to find triggers whose effective scope ends in a different
  node's spans. Violations are emitted against the affected nodes.

The cross-node check is deferred to `ADJ03a` because real-world clinical
prose tends to keep negation scope inside a sentence, and the simpler
per-node check covers the high-impact cases.

## Handling Compound Cues

Some cues are multi-word ("ruled out", "negative for", "no history of",
"family history of"). The trigger surface is a phrase rather than a
single token. The taxonomy supports phrase triggers via:

```text
Trigger {
    class:    RuleOut,
    surface:  "ruled out",     -- whitespace-separated tokens; matched as a phrase
    ...
}
```

Matching is case-insensitive by default but configurable per
domain. Stemming and lemmatization are deliberately not applied because
they introduce false positives in clinical text ("ruling" is not the
same as "ruled"; "negation" is not the same as "negative").

## Configuration Surface

Each adjudication ships with a *trigger taxonomy* — a versioned
configuration that lists the triggers, their classes, directions, and
scope rules. The framework's default taxonomy targets clinical text and
draws from NegEx and ConText releases that were validated against
clinical data (Chapman et al. 2001; Harkema et al. 2009). Other domains
will add their own (legal triggers like "shall not", "except where",
"provided that"; technical triggers like "deprecated", "removed in
version X").

The taxonomy and its version are recorded in the audit-trail metadata
for every adjudication. Re-running the check with a different taxonomy
version is a re-adjudication, not a re-check.

## RuledOut vs. Denied — The Distinction Is Real

A note specifically on the **distinction between RuledOut modality and
Denied polarity**, because collapsing them is a real and common semantic
error.

| Source phrasing | Polarity | Modality |
|---|---|---|
| "Patient denies chest pain." | Denied | Present |
| "No history of chest pain." | Denied | Past |
| "PE ruled out by CT angio." | Affirmed | RuledOut |
| "Concerning for PE." | Uncertain | Present |

"Denies" is the *patient's claim*. "Ruled out" is the *clinician's
adjudication*. Billing systems, malpractice review, and downstream
reasoning all treat these distinctly. The framework refuses to merge
them.

## Worked Examples

### Example 1: Negation caught

Source: *"Patient denies chest pain."*
Extractor output:
```text
Fact { term: chest_pain(patient), polarity: Affirmed,
       source_spans: [(doc1, 0, 28)] }
```

Polarity check:
- Find triggers in span. "denies" matches Negation class.
- Scope: forward to sentence end → covers "chest pain".
- Term's content word "chest_pain" is in scope.
- `required = polarity: Denied`, `actual = Affirmed`. **Fail.**
- Clarification: *"The span 'denies' indicates negation, but the IR is
  Affirmed. Is the patient denying chest pain, or affirming it?"*

### Example 2: Family history caught

Source: *"Father had MI at age 50."*
Extractor output:
```text
Fact { term: mi(patient), polarity: Affirmed, modality: Past,
       source_spans: [(doc1, 0, 24)] }
```

Polarity/modality check:
- Find triggers. "father" matches FamilyHistory.
- Scope: forward to sentence end.
- Term's content word "mi" is in scope.
- `required = modality: FamilyHistory`, `actual = Past`. **Fail.**
- Clarification: *"The span 'father' indicates a family-history
  attribution. The MI applies to a relative, not the patient. Re-extract
  with subject = father and modality = FamilyHistory."*

### Example 3: RuledOut correctly affirmed

Source: *"PE ruled out by CT angiography."*
Extractor output:
```text
Fact { term: pe(patient), polarity: Affirmed, modality: RuledOut,
       source_spans: [(doc1, 0, 30)] }
```

Polarity check:
- Find triggers. "ruled out" matches RuleOut class.
- Scope: backward over "PE".
- Term's content word "pe" is in scope.
- `required = modality: RuledOut`, `actual = RuledOut`. **Pass.**
- `required` does NOT touch polarity (RuleOut is modality only).
- Polarity remains Affirmed, which is correct.

This example shows why RuledOut is modality, not a polarity flip. A
naïve extractor might emit Denied here; the check catches it because
the RuleOut trigger class declares modality requirements only.

## Open Questions

1. **Conditional terms with structural conditions.** "Avoid X if patient
   has Y." Should the Conditional modality flag the node, and a separate
   node represent the condition? Probably yes; the structural shape is
   `Rule { head: avoid(X), body: [Pos(has(Y))] }` lowered. ADJ09 covers.
2. **Quoted speech inside notes.** "Patient said 'I don't drink alcohol'."
   The negation is inside a quoted attribution. Should the framework
   strip quotes or honor them? Current policy: honor them — the quoted
   span produces its own subject-attributed node.
3. **Stemming and lemmatization.** Deliberately omitted from the default
   taxonomy. Domains that need them may opt in via configuration.
4. **Trigger taxonomies for non-English.** All examples here are
   English. Generalization to other languages requires per-language
   trigger lists. Out of scope for the first paper.

## Limitations

1. The pass is a **scope detector**, not a syntactic parser. Genuinely
   syntactically subtle negation ("Although the chest pain has resolved,
   the patient remains uncomfortable.") may be miscategorized. The
   fall-through is ADJ04 (round-trip) and ADJ05 (adversarial).
2. **Domain coverage is finite.** A domain whose trigger taxonomy has
   gaps may produce false negatives. The fix is taxonomy iteration over
   adversarial corpora.
3. **Trigger conflicts.** Two overlapping triggers can produce
   contradictory required values. The framework reports both as
   violations and routes to clarification rather than guessing.

## Status

Draft. Sufficient to implement the per-node check directly. `ADJ03a`
(cross-node scope) and `ADJ03b` (taxonomy extensions for legal and
technical domains) are planned follow-ups.

# ADJ01 — Adjudication IR: Node Grammar, Type Rules, Lowering

## Overview

[`ADJ00`](ADJ00-adjudication-framework.md) introduced the framework and sketched the IR informally.
This spec defines the IR grammar formally: node shapes, the polarity and
modality lattices, the well-formedness conditions every node must satisfy,
the term language nodes are built out of, and the lowering edges that turn
high-level extractor output into the leaf nodes that the logic backends
actually consume.

The grammar is intentionally small. Six node kinds, two lattices, one DAG
structure. Everything else in the framework — the four checker passes
(`ADJ02..ADJ05`), the clarification protocol (`ADJ06`), the audit trail
(`ADJ07`), the rule-compilation pipeline (`ADJ09`) — composes over this
grammar without extending it.

A subsequent crate (`code/packages/rust/adjudication-ir`, to be created
parallel to `code/packages/python/adjudication-ir`) will implement the
grammar directly. This spec is the contract.

## Layer Position

```
   spec   ADJ00 framework overview
              │
              ▼
   spec   ADJ01 IR grammar              ← this document
              │
              ▼
   crate  adjudication-ir               ← reference implementation
              │
       ┌──────┴──────────────┐
       ▼                     ▼
   ADJ02..ADJ05         logic-core
   checker passes       (LP00, used as the term layer)
```

The IR's term language is layered directly on top of `logic-core` (see
[`LP00`](LP00-logic-core.md)). An IR node's `term` field holds an ordinary
`logic_core::Term`. The IR adds metadata around terms — polarity, modality,
source spans, provenance — that logic-core itself has no opinion about.

## Documents

The IR operates on **documents**. A document is the unit of adjudication: a
clinical note, a deal memo, a TSA carry-on declaration, a customer-support
ticket. Every IR node belongs to exactly one document at extraction time.

```text
DocumentId := opaque string (UUID v4 recommended; any unique identifier
                             acceptable provided it is stable across
                             clarification turns)

Span        := (DocumentId, start_offset, end_offset)
             where start_offset and end_offset are byte offsets into the
             document's normalized text representation.
```

Span offsets are byte offsets, not character indices, to avoid Unicode
normalization disagreements between extractor implementations. The
**normalized text** is whatever the document parser produces — Markdown
stripped to plain text, HTML rendered to text, OCR output cleaned — and
this normalization is recorded as document metadata so spans can be
re-resolved after re-normalization.

A document's lifecycle spans multiple clarification turns. New text appended
during clarification is part of the same document under a stable `DocumentId`,
with span offsets continuing into the appended region. Replay of a closed
adjudication recovers the document's exact byte sequence at each turn.

## The Term Language

The IR reuses the term language defined by `logic-core` (LP00):

```text
Term :=
    Atom(symbol)
  | Number(int | float)
  | String(value)
  | Var(LogicVar)
  | Compound(functor, args[])
```

No extensions. The IR's metadata is carried on the **node** that wraps a
term, never inside the term itself. This separation is important: it means
the existing logic backends consume IR-derived terms unchanged.

Lists, dates, quantities, and other "structured but not first-class" values
are encoded as compound terms following Prolog convention:

```text
[a, b, c]            →   '.'(a, '.'(b, '.'(c, [])))
2026-05-10           →   date(2026, 5, 10)
"100 ml"             →   quantity(100, ml)
"≤ 3.4 oz"           →   bound(le, 3.4, oz)
```

A canonical encoding registry for common structured values (dates,
quantities, units, ranges, durations) is part of `ADJ01a` (a forthcoming
sub-spec). It is referenced here so that two independent extractor
implementations agree on encoding.

## IR Nodes

Every IR node has the following fields:

```text
IRNode := {
    id:             NodeId,
    kind:           NodeKind,
    term:           Term,
    polarity:       Polarity,
    modality:       Modality,
    source_spans:   [Span],
    confidence:     Real in [0, 1],
    lowered_from:   Option<NodeId>,
    discard_reason: Option<DiscardReason>,
    metadata:       Map<string, Json>,
}

NodeId    := opaque string, unique within document
NodeKind  := Fact | Query | Uncertainty | Rule | Exception | Discarded
```

Field semantics:

| Field | Meaning |
|---|---|
| `id` | Stable identifier within the document. Used by `lowered_from`, by the proof DAG, and by replay tooling. |
| `kind` | The node's role in the IR. Determines which type rules apply. |
| `term` | The logical content (a `logic-core::Term`). |
| `polarity` | Whether the node asserts, denies, or is uncertain about `term`. |
| `modality` | The temporal / hypothetical / ownership context of `term`. |
| `source_spans` | The ranges of source text that produced this node. |
| `confidence` | Extractor's self-reported confidence. Informational only; not used by the type check. |
| `lowered_from` | If present, points to the parent node in the lowering DAG. |
| `discard_reason` | Required iff `kind = Discarded`. |
| `metadata` | Free-form extension. Reserved for downstream consumers; not consumed by the type check. |

The next four sections specify the lattices and the per-kind type rules.

## Polarity

```text
Polarity := Affirmed | Denied | Uncertain
```

Semantics:

- **Affirmed**: the node asserts `term`. "Patient has fever" → `Affirmed`.
- **Denied**: the node asserts `¬term`. "Patient denies chest pain" → `Denied`.
- **Uncertain**: the source neither asserts nor denies; the node records
  that the question was *raised* in the source. "Possible pneumonia" → an
  Uncertainty node with `Uncertain`. **Not** the same as low `confidence`.

The polarity lattice is flat: no element subsumes another. A node either
asserts, denies, or records uncertainty. There is no `Unknown` value; the
absence of evidence is represented by the *absence* of a node, not by a
node tagged Unknown — and the coverage check ensures any span that should
have produced a node did.

## Modality

```text
Modality := Present
          | Past
          | Future
          | Hypothetical
          | FamilyHistory
          | RuledOut
          | Conditional
```

Semantics:

- **Present**: holds at adjudication time. *"Patient currently has cough."*
- **Past**: held previously, no claim about now. *"History of asthma."*
- **Future**: anticipated. *"Will be discharged tomorrow."*
- **Hypothetical**: counterfactual or instructional. *"If the patient develops X..."*
- **FamilyHistory**: holds of a relative, not of the patient. *"Father had MI at 50."*
- **RuledOut**: explicitly excluded after consideration. *"PE ruled out by CT angio."*
  Distinct from `Denied` polarity: `Denied` is the source saying "no";
  `RuledOut` is the source saying "I considered this and excluded it." For
  diagnoses, this distinction matters for billing, audit, and downstream
  reasoning.
- **Conditional**: holds only when an attached condition is met. *"Avoid X
  if patient has Y."* The condition itself must be a separate node referenced
  by metadata.

`RuledOut` and `Denied` are deliberately separate because they are not
synonyms in clinical or legal practice. A clinician who *denies* writing a
prescription is making a different claim than a clinician who *ruled out*
prescribing one.

The modality lattice is also flat: no element subsumes another. Combining
modalities (e.g., past + family history) requires multiple nodes, not a
join in the lattice.

## Node Kinds and Their Type Rules

Each node kind has well-formedness constraints. The type check enforces
these structurally before any pass runs.

### Fact

A Fact node represents a state-of-the-world claim extracted from the input
(in the input pipeline) or a state-of-the-rulebook claim (in the rule
pipeline used by `ADJ09`).

Well-formedness:

```text
kind = Fact implies:
    polarity      != Uncertain         (Facts are not uncertain by construction)
    source_spans  is non-empty
    discard_reason is absent
    term          is a ground term, or a term containing only existentially
                  quantified variables (the variables are local to the node)
```

### Query

A Query node is a question the adjudication is asked to answer. Most
documents have exactly one Query, but multiple are permitted (e.g., "what's
the diagnosis AND what test to order next").

Well-formedness:

```text
kind = Query implies:
    polarity      = Affirmed            (Querying ¬p is itself a question)
    source_spans  is non-empty
    discard_reason is absent
    term          typically contains free variables — the answer binds them
```

### Uncertainty

An Uncertainty node records that the source explicitly raised a question
without answering it: "*possible* pneumonia," "*differential includes* PE."
Distinct from a low-confidence Fact.

Well-formedness:

```text
kind = Uncertainty implies:
    polarity      = Uncertain
    source_spans  is non-empty
    discard_reason is absent
```

Uncertainty nodes are consumed by the probabilistic backend (`ADJ11`) as
hints about which facts to treat as variables of interest; in the
non-probabilistic adjudication path they surface to clarification.

### Rule

A Rule node represents a piece of the rulebook compiled into the IR. Rules
are produced by the rule-compilation pipeline (`ADJ09`), not the
input-extraction pipeline.

Well-formedness:

```text
kind = Rule implies:
    polarity      != Uncertain
    source_spans  is non-empty       (must cite the rulebook span)
    discard_reason is absent
    term          is a compound term whose functor is one of:
                      definitional( head, body[] )
                      constraint( body[] )
                      default( head, body[], exceptions[] )
                      probabilistic( probability, head, body[] )
    metadata MUST contain an `as_of` ISO-8601 date stamp from the source.
```

The Rule subtype is encoded in the term, not in the kind, to avoid an
explosion of kinds while preserving the structural type check. See
[`ADJ09`](ADJ09-rule-compilation-pipeline.md) — TBD — for the detailed
typing of each Rule subtype.

### Exception

An Exception node is a structured carve-out attached to a Rule. Distinct
from a separate Rule because Exceptions are syntactically lexically scoped
to their parent Rule.

Well-formedness:

```text
kind = Exception implies:
    polarity      = Affirmed
    source_spans  is non-empty
    discard_reason is absent
    metadata MUST contain `applies_to: NodeId` pointing to a Rule node.
```

Exceptions and the priority order between rules are first-class because
real rulebooks (medical guidelines, tax code, license terms) routinely
declare exceptions explicitly.

### Discarded

A Discarded node represents a span of input that the extractor deliberately
chose not to translate into a Fact / Query / Uncertainty / Rule.

Well-formedness:

```text
kind = Discarded implies:
    discard_reason is non-empty
    source_spans   is non-empty
    polarity       = Affirmed         (we affirm the discard, not deny content)
    modality       = Present
    term           is the atom `discarded`
```

`DiscardReason` is drawn from a controlled vocabulary so coverage analysis
can audit *why* spans were dropped:

```text
DiscardReason :=
    Pleasantry            -- "Hi doc, hope you had a good weekend."
    DocumentMetadata      -- header / footer / boilerplate
    NonDomainContent      -- patient asked about parking
    Restatement           -- duplicates an already-extracted fact
    Unparseable           -- text whose meaning the extractor cannot determine
    AdministrativeOnly    -- billing codes, MRN, etc. captured elsewhere
    ExplicitlyOutOfScope  -- e.g., "I'll address this in next visit"
```

The `Unparseable` reason is *always* a coverage failure: an extractor that
declares text Unparseable triggers a clarification rather than shipping the
node. This is enforced by `ADJ02`.

## The Lowering DAG

The IR is a directed acyclic graph induced by `lowered_from` edges.

```text
A node X is a leaf if no other node Y has lowered_from = X.
A node X is a root if X.lowered_from = None.
```

Roots are produced directly by the extractor from the source. Leaves are
what the logic backends consume. Intermediate nodes record refinement steps
— a high-level extraction that was subsequently split, made more specific,
or restated more formally during clarification.

```text
For every non-root node X:
    X.lowered_from = Y  implies
        Y.source_spans is a (not necessarily strict) superset of X.source_spans
        Y.kind is "compatible with" X.kind (see lowering rules below)
```

The "spans superset" condition guarantees that lowering only narrows
provenance, never invents it. A leaf node's source spans are always
contained within its root's spans, transitively.

### Lowering Rules

The kind-compatibility relation `≺` for lowering is:

```text
Fact            ≺ Fact            (refinement: same claim, more specific)
Uncertainty     ≺ Fact            (resolution: clarification turned uncertainty
                                   into an affirmation or denial)
Uncertainty     ≺ Uncertainty     (refinement: more specific uncertain claim)
Query           ≺ Query           (decomposition: one query broken into
                                   subqueries)
Rule            ≺ Rule            (refinement during compilation)
```

Every other combination is forbidden. A Discarded node cannot be lowered;
a node cannot be lowered *to* Discarded (use the type check instead).

A Fact may NOT lower to an Uncertainty. The framework treats clarification
that *introduces* uncertainty as a coverage check failure: the extractor
should have produced an Uncertainty at the root in the first place.

## Well-Formedness Summary

An IR document is well-formed iff:

1. Every node satisfies its kind-specific constraints (above).
2. `lowered_from` forms a DAG (no cycles).
3. For every non-root node X with `X.lowered_from = Y`:
   - `Y` exists in the document.
   - `kind(Y) ≺ kind(X)` per the lowering rules.
   - `X.source_spans ⊆ Y.source_spans`.
4. Every Exception node's `applies_to` metadata points to an existing Rule
   in the same document.
5. Every Span's offsets are valid byte ranges in the document at the turn
   the node was produced.

The `ADJ01-ref` Rust crate (`code/packages/rust/adjudication-ir`) will
implement a `validate()` function that enforces all five conditions and
returns the specific violation when any fails. Validation is total — no
partial well-formedness.

## Serialization

The on-disk and on-wire form is JSON. A JSON Schema for `IRNode` lives at
`code/specs/schemas/adjudication-ir.schema.json` (to be added when the
Rust crate lands).

The schema mirrors the grammar above, with two presentation choices:

- `term` is encoded using the canonical s-expression-like form already used
  by other logic-vm tooling in this repo, *not* as a nested JSON object.
  This keeps round-tripping through external tools (Prolog dumps, ProbLog
  programs) lossless.
- `metadata` is a free-form JSON object; downstream consumers may add keys
  but the framework reserves any key beginning with `adj.` for future use.

A reference round-trip (a parsed IR document re-emitted as JSON equals the
input byte-for-byte after canonicalization) is part of the test suite for
the Rust crate.

## Worked Example: TSA, Lowered

Continuing the running example from `ADJ00`. The input *"I'd like to bring
a 4 oz tube of toothpaste, ... I am not bringing matches, only a single
disposable lighter."* produces the root nodes already shown in `ADJ00`.
This section traces one lowering step.

Root node, produced by the extractor:

```text
F6 {
    id:           "F6",
    kind:         Fact,
    term:         carry_on_item(matches),
    polarity:     Denied,
    modality:     Present,
    source_spans: [(doc1, 142, 168)],   // "I am not bringing matches"
    confidence:   0.93,
    lowered_from: None,
}
```

The polarity check (`ADJ03`) examines span `(doc1, 142, 168)`, finds the
negation trigger "not bringing", confirms it is in scope of the noun
"matches", and accepts polarity = Denied. Pass succeeds.

Now suppose the clarification dialogue asks (because matches are not a
single category in TSA rules — safety matches and strike-anywhere matches
have different rules) and the user responds *"safety matches."* The
extractor produces a lowered node:

```text
F6a {
    id:           "F6a",
    kind:         Fact,
    term:         carry_on_item(safety_matches),
    polarity:     Denied,
    modality:     Present,
    source_spans: [(doc1, 142, 168), (doc1, 245, 261)],
                  // original span + clarification response span
    confidence:   0.98,
    lowered_from: Some("F6"),
}
```

The well-formedness checks: `F6a.lowered_from = F6` exists; `kind(F6) =
Fact ≺ Fact = kind(F6a)` is allowed; `F6a.source_spans ⊇ F6.source_spans`
is satisfied. Validation passes.

The logic engine consumes `F6a` (the leaf), not `F6`. The audit trail
shows the lowering chain so a reviewer can see why the leaf has the spans
it does.

## Open Questions

The following are deferred to subsequent revisions of this spec:

1. **Quantifier semantics for variables inside Fact terms.** Existential is
   the default; universal-in-fact-terms is rare but real ("all of patient's
   medications include X"). Probably a metadata tag rather than a new kind.
2. **Negation as failure vs. classical negation in rule bodies.** Cleanly
   defined in ProbLog (well-founded semantics); needs explicit handling
   when Rule terms contain negative literals. Punted to `ADJ09`.
3. **Cross-document references.** A clinical note may reference a prior
   note; a deal memo may reference a contract. Current scope is single-
   document; cross-document is `ADJ08` (replay tooling) and beyond.
4. **Anaphora resolution.** "The patient" → which patient? Currently
   delegated to the extractor as a pre-processing step; the IR sees
   resolved entities. May need to surface unresolved anaphora as
   Uncertainty nodes if extractors disagree.
5. **Streaming documents.** The spec assumes complete documents at
   extraction time. Streaming (e.g., real-time scribing during a visit)
   is out of scope for the first paper but worth thinking about for v2.

## Limitations

This spec defines the IR structurally. It does not define:

1. **How a particular LLM should be prompted** to produce well-formed IR
   nodes. Extractor prompts are an implementation choice and likely vary
   per base model. The framework only cares whether the IR validates.
2. **How clarification questions should be phrased.** That's `ADJ06`.
3. **Probabilistic semantics for the IR.** Probabilities on Rules are a
   number field; their meaning is given by ProbLog distribution semantics,
   specified in `ADJ11`.
4. **Performance characteristics.** Nothing in the IR forces a particular
   representation; the Rust reference crate will pick one (likely
   `serde`-friendly structs with `Arc<Term>` for sharing) but the spec
   does not mandate it.

## Status

Draft. The grammar is intended to be stable enough to implement against;
small revisions are likely as the Rust reference crate exposes
under-specified corners. Versioned via the surrounding repo's commit
history; the `adjudication-ir` crate will declare a `schema_version` field
on every emitted document so old IRs remain parsable as the grammar
evolves.

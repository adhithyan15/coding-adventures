# ADJ01 — Adjudication IR: Hierarchical Decomposition, Node Grammar, Lowering

> **Revision v2 (2026-05-11): hierarchical decomposition.** The first
> version of this spec defined a flat IR whose validation depended on
> a language-specific token tagger (`ADJ02`) and a NegEx/ConText-style
> trigger taxonomy (`ADJ03`). Both choices baked English-language
> assumptions into the framework. This revision replaces those
> assumptions with **LLM-driven hierarchical decomposition**: the
> extractor produces a *tree* whose leaves are the existing IR kinds
> (`Fact`, `Query`, `Uncertainty`, `Rule`, `Exception`, `Discarded`)
> and whose internal nodes are a new `TextRun` kind that exists only
> to carry the decomposition. Coverage becomes a **structural tree
> check** (`ADJ02-v2`); polarity / modality consistency becomes a
> **propagation check** (`ADJ03-v2`). No language-specific knowledge
> lives in the framework. See `ADJ02` and `ADJ03` for the updated
> checker semantics.

## Overview

[`ADJ00`](ADJ00-adjudication-framework.md) introduced the framework
and sketched the IR informally. This spec defines the IR grammar
formally: node shapes, the polarity and modality lattices, the
hierarchical decomposition tree, the lowering DAG, and the
well-formedness conditions every node must satisfy.

The grammar is intentionally small. Seven node kinds (one of them
new in v2), two lattices, a decomposition tree, and a lowering DAG.
Everything else in the framework — the four checker passes
(`ADJ02..ADJ05`), the clarification protocol (`ADJ06`), the audit
trail (`ADJ07`), the rule-compilation pipeline (`ADJ09`) — composes
over this grammar without extending it.

## Why a Hierarchical Tree

The framework's central claim is that the LLM does the *linguistic*
work and the checker passes do the *structural* verification. A flat
IR cannot uphold that claim: deciding whether a token is "meaningful"
or whether a span carries negation requires linguistic knowledge that
the framework would have to encode itself, locking the design to a
single language (and, in practice, the English clinical idiom).

A hierarchical IR pushes all that knowledge into the LLM. The LLM
breaks the input into a tree of text runs, refining toward atomic
claims; each level is the LLM's report on what the input means at
that granularity. The framework's job is only to verify the
**tree's structural invariants**:

- Every byte of the input is in some leaf's source spans
  (structural coverage).
- Each parent's polarity / modality is consistent with its
  children's effective values (propagation consistency).
- The leaves are well-formed nodes of the existing kinds.

These checks are language-agnostic by construction. The trigger
taxonomies and stopword lists of v1 disappear from the framework
core; they may live on as optional accelerators for narrow domains
where rule-based shortcuts are demonstrably reliable.

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
   ADJ02-v2..ADJ05      logic-core
   structural / LLM     (LP00, used as the term layer)
   checker passes
```

The IR's term language is layered directly on top of `logic-core`
(see [`LP00`](LP00-logic-core.md)). A leaf IR node's `term` field
holds an ordinary `logic_core::Term`. The IR adds metadata around
terms — polarity, modality, source spans, structural and refinement
edges — that logic-core itself has no opinion about.

## Documents

The IR operates on **documents**. A document is the unit of
adjudication: a clinical note, a deal memo, a TSA carry-on
declaration, a customer-support ticket.

```text
DocumentId := opaque string (UUID v4 recommended; any unique
                             identifier acceptable provided it is
                             stable across clarification turns)

Span        := (DocumentId, start_offset, end_offset)
             where start_offset and end_offset are byte offsets into
             the document's normalized text representation.
```

Byte offsets, not character indices, to avoid Unicode normalization
disagreements between implementations. The **normalized text** is
whatever the document parser produces and is recorded as document
metadata so spans can be re-resolved after re-normalization.

A document's lifecycle spans multiple clarification turns. New text
appended during clarification is part of the same document under a
stable `DocumentId`.

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

No extensions. IR metadata is carried on the **node** that wraps a
term, never inside the term itself. This keeps the existing logic
backends consuming IR-derived terms unchanged.

Lists, dates, quantities, and other structured values are encoded as
compound terms following Prolog convention:

```text
[a, b, c]            →   '.'(a, '.'(b, '.'(c, [])))
2026-05-10           →   date(2026, 5, 10)
"100 ml"             →   quantity(100, ml)
```

## IR Nodes

Every IR node has the following fields:

```text
IRNode := {
    id:             NodeId,
    kind:           NodeKind,
    term:           Term,                  -- meaningful for non-TextRun kinds
    polarity:       Polarity,
    modality:       Modality,
    source_spans:   [Span],
    confidence:     Real in [0, 1],
    part_of:        Option<NodeId>,        -- NEW IN v2: structural parent
    lowered_from:   Option<NodeId>,        -- refinement (clarification) parent
    discard_reason: Option<DiscardReason>, -- required iff kind = Discarded
    metadata:       Map<string, Json>,
}

NodeId    := opaque string, unique within document
NodeKind  := TextRun                       -- NEW IN v2: non-leaf decomposition node
           | Fact | Query | Uncertainty | Rule | Exception | Discarded
```

### `part_of` (new in v2): the structural decomposition edge

`part_of` points to the **parent** in the decomposition tree. A node
with `part_of = None` is at the document root. A `TextRun` parent's
**children** are every node whose `part_of` equals the parent's id.

The structural-coverage invariant (enforced by `ADJ02-v2`) requires
that the union of a parent's children's `source_spans` equals the
parent's `source_spans`. Every byte of the document ends up in some
leaf, transitively.

### `lowered_from` (unchanged): the refinement DAG edge

`lowered_from` points to a node that this one **refines** —
typically a more general claim that a clarification turn made
specific. `lowered_from` is independent of `part_of`. A node can
have both (it has a structural parent *and* refines an earlier
version of itself). Refinement is a DAG (no cycles); the
decomposition is a tree.

### Term for `TextRun`

`TextRun` nodes don't carry a domain claim — they exist only to
group children. Their `term` field is required (the IRNode shape is
uniform) and conventionally set to `text_run/0` (the zero-arity
compound) or to a domain-specific description. Validators don't
inspect it.

## Hierarchical Decomposition

```text
Document (whole input, source_spans = [(doc, 0, len)])
   │
   ▼ part_of edges (children point to this root)
   │
   ├── TextRun  (a section / paragraph)
   │     ├── TextRun (a sentence)
   │     │     ├── Fact     ← leaf
   │     │     └── Discarded ← leaf (a non-domain phrase)
   │     └── Fact
   ├── TextRun
   │     └── Uncertainty
   └── Discarded   (a top-level non-domain span)
```

The root has `part_of = None`. Every other node has `part_of` set to
its structural parent. Leaves are nodes whose kind is not `TextRun`
(though `Discarded` can also appear at non-leaf positions when a
whole sub-tree of text is non-domain).

**Two related design choices worth flagging:**

1. **`Document` is not its own kind.** The document is represented
   by whichever node(s) have `part_of = None`. Most documents have
   one root (a single TextRun covering the whole input); split-root
   documents (e.g., a clinical note with separate History and
   Examination sections) are valid as long as every root collectively
   tiles the document's bytes.
2. **Leaves carry the term; TextRuns don't.** All claim content
   lives at the leaves. TextRuns carry source spans and (optionally)
   polarity/modality that propagates to descendants.

## Polarity and Modality on TextRuns: Propagation

A non-leaf `TextRun` can carry a polarity and modality that **applies
to every leaf descendant unless overridden**. The canonical example:

```text
TextRun (polarity = Denied)        spans = [(doc, 0, 50)]
  ↓ "Patient denies the following: chest pain, fever, shortness of breath."
  Fact term=chest_pain(p)  polarity = Denied (inherited)
  Fact term=fever(p)        polarity = Denied (inherited)
  Fact term=sob(p)          polarity = Denied (inherited)
```

The LLM emits the parent's polarity once and propagates to children
implicitly. Children may override by carrying their own non-`Inherit`
value. Propagation rules:

- If a leaf carries an explicit (non-default) polarity, that value
  wins.
- Otherwise, the nearest ancestor with a non-default polarity wins.
- Modality propagates by the same rule.
- The framework's default values are `Affirmed` for polarity and
  `Present` for modality. A node that means "use the inherited
  value" sets these defaults; one that means "override" sets a
  different value.

`ADJ03-v2` formalises the propagation algorithm and the check that
the result is consistent.

## Polarity

```text
Polarity := Affirmed | Denied | Uncertain
```

Unchanged from v1. Flat lattice; no `Unknown`. The absence of
evidence is the absence of a node, enforced by the structural
coverage check.

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

Unchanged from v1. Flat lattice. `RuledOut` and `Denied` (a polarity
value) remain explicitly distinct because clinical and legal
practice treats them as non-synonyms.

## Node Kinds and Their Type Rules

### TextRun (new in v2)

A non-leaf node that exists to carry the decomposition.

Well-formedness:

```text
kind = TextRun implies:
    source_spans          is non-empty (must cite document bytes)
    discard_reason        is absent
    at least one node has part_of = this.id     -- non-leaf by construction
```

### Fact

A claim about the world.

Well-formedness (unchanged from v1):

```text
kind = Fact implies:
    polarity      != Uncertain
    source_spans  is non-empty
    discard_reason is absent
```

### Query

A question the adjudication is asked.

Well-formedness:

```text
kind = Query implies:
    polarity      = Affirmed
    source_spans  is non-empty
    discard_reason is absent
```

### Uncertainty

The source explicitly raised a question without answering it.

```text
kind = Uncertainty implies:
    polarity      = Uncertain
    source_spans  is non-empty
    discard_reason is absent
```

### Rule

Produced by the rule-compilation pipeline (`ADJ09`).

```text
kind = Rule implies:
    polarity      != Uncertain
    source_spans  is non-empty
    discard_reason is absent
    metadata MUST contain `as_of` (ISO-8601)
    term          is a compound term whose functor is one of:
                      definitional( head, body[] )
                      constraint( body[] )
                      default( head, body[], exceptions[] )
                      probabilistic( probability, head, body[] )
```

### Exception

A carve-out attached to a Rule.

```text
kind = Exception implies:
    polarity      = Affirmed
    source_spans  is non-empty
    discard_reason is absent
    metadata MUST contain `applies_to: NodeId` pointing to a Rule node.
```

### Discarded

An explicit span the extractor judged irrelevant.

```text
kind = Discarded implies:
    discard_reason is non-empty
    source_spans   is non-empty
    polarity       = Affirmed
    modality       = Present
    term           is the atom `discarded`
```

Controlled `DiscardReason` vocabulary unchanged:

```text
DiscardReason :=
    Pleasantry
  | DocumentMetadata
  | NonDomainContent
  | Restatement
  | Unparseable            -- always a coverage failure
  | AdministrativeOnly
  | ExplicitlyOutOfScope
```

## The Decomposition Tree (replaces "The Lowering DAG" section from v1)

The IR has two distinct edge sets:

1. **Decomposition tree** (`part_of`): structural. The whole hierarchy
   from document roots to leaves. Every non-root node has exactly one
   structural parent.
2. **Refinement DAG** (`lowered_from`): historical / refinement.
   Records that a node is a more-specific version of an earlier node,
   typically created via clarification.

The two are orthogonal:

- A node may have a `part_of` (it's in the structural tree) and a
  `lowered_from` (it's a refinement of an earlier leaf).
- A leaf created during clarification typically has `part_of` set to
  the same parent as its predecessor (the refinement happens
  *inside* the structural slot).

### Structural-tree well-formedness

For every node X with `X.part_of = Some(Y)`:

- `Y` exists in the document.
- `Y.kind = TextRun` (only TextRuns can have children; leaves cannot).
- `X.source_spans` ⊆ `Y.source_spans` (children fit inside their
  parent's spans).

For every TextRun Y:

- Let `C(Y)` = `{X : X.part_of = Some(Y)}`.
- The union of `C(Y)`'s `source_spans` must equal `Y.source_spans`.
  Every byte of the parent's spans is in some child's spans.

This is the **structural coverage invariant**, formalised in
`ADJ02-v2`.

### Refinement-DAG well-formedness (unchanged from v1)

For every node X with `X.lowered_from = Some(Y)`:

- `Y` exists.
- `Y.kind` is "compatible with" `X.kind` per the kind-compatibility
  relation: `Fact ≺ Fact`, `Uncertainty ≺ Fact`, `Uncertainty ≺
  Uncertainty`, `Query ≺ Query`, `Rule ≺ Rule`.
- `X.source_spans` ⊆ `Y.source_spans` (refinement narrows
  provenance).

The refinement DAG is acyclic.

## Well-Formedness Summary

An IR document is well-formed iff:

1. Every node satisfies its kind-specific constraints (above).
2. Every node id is unique.
3. The decomposition tree (`part_of` edges) is a forest of trees:
   each non-root node has exactly one structural parent; no cycles.
4. Every TextRun's children's spans tile its spans (structural
   coverage invariant).
5. The refinement DAG (`lowered_from` edges) is acyclic and obeys
   the kind-compatibility relation.
6. For every non-root node X with `X.part_of = Some(Y)`,
   `X.source_spans ⊆ Y.source_spans`.
7. Every Exception node's `applies_to` metadata points to an
   existing Rule.
8. Every Span's offsets are valid byte ranges in the document.

`validate()` enforces all eight conditions and returns the specific
violation when any fails. Validation is total — no partial
well-formedness.

## Lowering Rules (Kind Compatibility for Refinement)

Unchanged from v1:

```text
Fact            ≺ Fact            (refinement: same claim, more specific)
Uncertainty     ≺ Fact            (resolution via clarification)
Uncertainty     ≺ Uncertainty     (more specific uncertain claim)
Query           ≺ Query           (decomposition into subqueries)
Rule            ≺ Rule            (rule refinement during compilation)
```

A Fact may **not** lower to an Uncertainty. The framework treats
clarification that introduces uncertainty as a structural failure:
the LLM should have produced an Uncertainty leaf at the original
position.

`TextRun` does not participate in the refinement DAG — it's a
structural-only kind.

## Worked Example (Hierarchical Form)

Continuing the TSA example. The input *"I'd like to bring a 4 oz
tube of toothpaste, ..., I am not bringing matches, only a single
disposable lighter."* now decomposes hierarchically:

```text
N0 TextRun     "I'd like to bring ... disposable lighter."   (doc, 0, 209)
   │
   ├── N1 TextRun   "I'd like to bring ... lighter."         (doc, 0, 149)
   │     │
   │     ├── F1 Fact carry_on_item(toothpaste, ...) (doc, 5, 45)
   │     ├── F2 Fact carry_on_item(perfume, ...)    (doc, 46, 62)
   │     ├── F3 Fact carry_on_item(lithium_battery,...) (doc, 63, 93)
   │     ├── F4 Fact carry_on_item(wine, ...)       (doc, 94, 124)
   │     └── F5 Fact carry_on_item(pocket_knife,...) (doc, 125, 149)
   │
   ├── N2 TextRun polarity=Denied  "I am not bringing matches" (doc, 150, 176)
   │     │
   │     └── F6 Fact carry_on_item(matches)         (doc, 150, 176)
   │           (inherits Denied from N2)
   │
   └── N3 TextRun     "only a single disposable lighter."   (doc, 177, 209)
         │
         └── F7 Fact carry_on_item(lighter, ...)   (doc, 177, 209)
```

The negation polarity for F6 is **carried by the TextRun parent N2**,
not detected by scanning F6's span for the word "not". The framework
verifies that F6's effective polarity (the propagated Denied) matches
its declared polarity (also Denied or `Inherit` — either way
consistent). No NegEx machinery is involved. The LLM saw the phrase
*"I am not bringing matches"* and chose to emit a parent TextRun with
Denied polarity; that decision is in the IR, auditable, and
verifiable structurally.

## Serialization

The on-disk and on-wire form is JSON. A JSON Schema for `IRNode` will
live at `code/specs/schemas/adjudication-ir.schema.json` (to be
added when the Rust crate's v2 lands).

Schema additions in v2:

- `kind` enum gains `"TextRun"`.
- `part_of` field, optional NodeId.
- `polarity` and `modality` gain an `"Inherit"` value indicating
  "use the parent's effective value." Default for TextRun children.

## Open Questions (revised)

1. **Default vs. Inherit ambiguity.** `polarity: Affirmed` and
   `polarity: Inherit` are different declarations but produce the
   same effective value when the ancestor chain is all Affirmed.
   Should the extractor be required to be explicit? The current
   design accepts either; the audit trail records the literal field.
2. **Sibling polarity disagreement.** If two siblings declare
   `Affirmed` and `Denied` under a `Denied` parent, the propagation
   succeeds (the Affirmed child overrides). Whether that pattern is
   "natural" (a contrast) or "an error" depends on the source text.
   `ADJ03-v2` will likely raise it for clarification but not fail
   the structural check.
3. **Multi-root documents.** Most documents have one root; sectioned
   documents (clinical notes with separate sections) have several.
   The framework supports both; deployments may prefer one
   convention.
4. **Discarded TextRuns vs. Discarded leaves.** A whole sub-tree
   that is non-domain content can be represented either as a single
   `Discarded` leaf at the right level OR as a TextRun with a
   Discarded leaf inside. Both are well-formed; the LLM picks.

## Limitations (revised)

1. **The LLM is doing more work** than in v1. Polarity decisions,
   scope detection, sentence segmentation, family-history
   recognition — all of these are now the LLM's responsibility.
   The framework verifies the *output*, not the *process*.
2. **Language-agnosticism in theory, model-agnosticism in
   practice**. The LLM must be competent in the input language.
   Frontier models handle major languages well; low-resource
   languages remain a deployment concern.
3. **The flat-IR optimisation path of v1** (rule-based tagger +
   trigger taxonomy for narrow English-only domains) is retired
   from the framework core. It may re-emerge as an optional
   `domain-language-helpers` accelerator crate for deployments where
   the rule-based approach is empirically fast and good enough.
4. **Backwards compatibility**. v1 IR documents do not satisfy v2's
   structural-coverage invariant unless wrapped in a single TextRun
   covering the whole document. A migration helper for v1 → v2
   trivially adds such a wrapper.

## Status

v2 draft. The hierarchical decomposition replaces the
language-specific rule-based path of v1 across `ADJ02`, `ADJ03`, and
the `adjudication-coverage` / `adjudication-polarity-modality` Rust
crates. Those specs and crates are being revised in parallel; see
`ADJ02` and `ADJ03` for the updated checker semantics.

# ADJ01 — Adjudication IR: Multi-Directed Acyclic Graph, Node and Edge Grammar, Lowering

> **Revision v3 (2026-05-12): graph IR.** v2's hierarchical decomposition
> tree (`part_of` edges to a single structural parent, `lowered_from`
> edges in a separate refinement DAG, `TextRun` for non-leaf
> grouping) becomes a single **multi-directed acyclic graph** of typed
> nodes and typed edges. The hierarchical tree was sufficient for
> small narratives (a single clinical note, a 200-byte TSA
> declaration) but is structurally incapable of representing the
> relationships that emerge at scale: rule citations, cross-document
> references, table-row membership, multi-rule provenance,
> exception scoping, temporal supersession. v3 replaces the tree with
> typed edges so those relationships are first-class IR objects
> rather than text that the checker passes have to re-discover.
>
> **Backwards compatibility**: there is none. v2 IRs are not v3 IRs.
> Nothing in this repository ships yet; the migration cost is in the
> framework code, not in deployed adjudications. See
> [`ADJ09`](ADJ09-rule-compilation-pipeline.md) and
> [`ADJ14`](ADJ14-rule-elicitation.md) for the downstream specs that
> motivated v3.

## Overview

[`ADJ00`](ADJ00-adjudication-framework.md) introduced the framework
and sketched the IR informally. This spec defines the v3 IR formally:
node shapes, edge shapes, the typed edge-relation taxonomy, the
polarity and modality lattices, propagation through the graph, the
coverage and acyclicity invariants, and the well-formedness conditions
every IR must satisfy.

The grammar deliberately stays small: seven node kinds, one edge
shape with an enumerated relation, two lattices, a flat byte-tiling
coverage rule, and a single DAG invariant. Everything else in the
framework — the four checker passes (`ADJ02..ADJ05`), the
clarification protocol (`ADJ06`), the audit trail (`ADJ07`), the
rule-compilation pipeline (`ADJ09`), the rulebook elicitation
phase (`ADJ14`) — composes over this grammar without extending it.

## Why a Graph IR

The framework's central claim is that the LLM does the *linguistic*
work and the checker passes do the *structural* verification. v2's
hierarchical tree pushed all linguistic decisions to the LLM and
verified only structural tree invariants — a real improvement over
v1's English-specific tagger. But the tree shape limits what the
framework can verify.

Consider the following relationships that appear naturally in
adjudicable text but cannot be expressed by a tree:

| Relationship | Source | Tree fails because… |
|---|---|---|
| Exception E1 modifies Rule R3 | `"...except for crew members"` | E1 is structurally a sibling of R3 inside the section, not its child. The modification is *semantic*, not structural. |
| Rule R7 cites authority "49 CFR § 1540.111" | `"...per 49 CFR § 1540.111(a)..."` | Authority is a distinct entity referenced from R7. A tree must inline it or lose it. |
| "see §5" cross-reference | `"...as defined in §5"` | A tree has no slot for a directed link to another part of the document. |
| Tabular row | a drug-dosage row | A row is one fact set; its cells aren't its children — they're its co-equals at a structural level. |
| Provenance chain | answer is justified by [R1, R2, R3] | A chain across rules is a path, not a parent-child relationship. |
| Rule R' supersedes Rule R | a 2024 amendment | Supersession crosses document versions; trees within a single document cannot express it. |

A graph IR represents each as a first-class edge with a typed
relation. The checker passes verify that the edges are themselves
well-formed (acyclic, covered, polarity-consistent) without
re-discovering the relationships from text. The audit trail records
*both* nodes *and* edges, so an adjudication's provenance is exactly
the set of nodes and edges that participated.

**The cost** is that the LLM has more shapes to emit and the validator
has more invariants to check. The benefit is that everything
downstream — rule compilation, expert review, engine lowering,
replay — operates on the explicit structure rather than reverse-engineering
it from a tree.

## Layer Position

```
   spec   ADJ00 framework overview
              │
              ▼
   spec   ADJ01 IR grammar (v3 — this document)
              │
              ▼
   crate  adjudication-ir              ← reference implementation
              │
       ┌──────┴──────────────┐
       ▼                     ▼
   ADJ02..ADJ05         logic-core
   coverage / polarity   (LP00, term layer)
   round-trip / adversarial
```

The IR's term language continues to layer on top of `logic-core` (see
[`LP00`](LP00-logic-core.md)). A node's `term` field still holds an
ordinary `logic_core::Term`. v3 changes how nodes *relate*, not the
language of what they *say*.

## Documents

Unchanged from v2.

```text
DocumentId := opaque string (UUID v4 recommended; any stable unique
                             identifier acceptable across clarification
                             turns)

Span        := (DocumentId, start_offset, end_offset)
             where the offsets are byte offsets into the document's
             normalized text representation.
```

Byte offsets, not character indices, to avoid Unicode normalization
disagreements between implementations.

A document's lifecycle spans multiple clarification turns. New text
appended during clarification is part of the same document under a
stable `DocumentId`.

## The Term Language

Unchanged from v2:

```text
Term :=
    Atom(symbol)
  | Number(int | float)
  | String(value)
  | Var(LogicVar)
  | Compound(functor, args[])
```

No extensions. IR metadata is carried on the **node or edge** that
wraps a term, never inside the term itself.

Lists, dates, quantities, and other structured values are encoded as
compound terms following Prolog convention.

## IR Nodes

```text
IRNode := {
    id:             NodeId,
    kind:           NodeKind,
    term:           Term,
    polarity:       Polarity,
    modality:       Modality,
    source_spans:   [Span],
    confidence:     Real in [0, 1],
    discard_reason: Option<DiscardReason>,   -- required iff kind = Discarded
    metadata:       Map<string, Json>,
}

NodeId    := opaque string, unique within document
NodeKind  := Fact | Query | Uncertainty
           | Rule | Exception | Discarded
           | Section | Entity
```

**What changed from v2:** the `part_of` and `lowered_from` fields are
removed. The decomposition tree and the refinement DAG both move into
the typed edge layer (with relations `Contains` and `Clarifies`
respectively). `TextRun` is replaced by `Section`, which carries
meaningful structural metadata (paragraph, section, table, row) rather
than being a content-free grouping node.

A new kind `Entity` is added for deduplicated reference targets:
when the same atom is mentioned at multiple byte ranges, a single
`Entity` node represents the thing, and each mention is a separate
`Mentions` edge from the mentioning node to the Entity.

## IR Edges

```text
IREdge := {
    id:             EdgeId,
    source:         NodeId,
    target:         NodeId,
    relation:       EdgeRelation,
    polarity:       Polarity,
    modality:       Modality,
    source_spans:   [Span],
    confidence:     Real in [0, 1],
    metadata:       Map<string, Json>,
}

EdgeId       := opaque string, unique within document
```

Edges are **directed**: `source` and `target` are not symmetric.
Multiple edges may exist between the same pair of nodes provided they
have different `relation` values (hence *multi*-directed). Edge
polarity and modality allow negated or conditional edges:

- `Excepts(E, R)` with polarity `Affirmed` — E modifies R.
- `Excepts(E, R)` with polarity `Denied` — E was *explicitly excluded*
  as an exception to R (e.g., "the standard exception E does not apply
  to R"). The audit trail still records it.
- `AppliesTo(R, Entity)` with modality `Conditional` — R applies to
  Entity only under some condition (the condition itself is in
  another edge or in R's term body).

Edges carry source_spans for the **textual marker that signals the
relation**, not for the related nodes themselves. For "Rule R5 except
for crew members", the `Excepts` edge's source_spans cover the bytes
of "except for". The "crew members" entity has its own spans on the
Entity node; "Rule R5" has its spans on the Rule node.

A synthesized edge (one added by the engine, not extracted from text —
e.g., a `JustifiedBy` edge in an answer's provenance) carries
**empty** source_spans. It's still recorded in the audit trail; it
just doesn't tile any source bytes because it didn't come from any.

## The Edge-Relation Taxonomy

Closed set. Adding a new relation is a v-bump (v4 etc.) so checkers
and engine lowering can rely on the relation set being known. The
escape hatch `DomainSpecific(name)` accommodates deployments that need
a relation the framework doesn't yet ship without forcing a v-bump.

Eleven groups, organized by what the relation describes:

### 1. Structural — document and section organization

| Relation | Direction | Meaning |
|---|---|---|
| `Contains` | parent → child | The source structurally contains the target. Section→Fact, Section→Section, Table→Row, Row→Cell. |
| `Precedes` | earlier → later | Source appears before target in document order. Section S1 → Section S2. |
| `Heading` | heading → body | Source's spans are the heading text; target is the body it labels. |

### 2. Identity — deduplication and reference resolution

| Relation | Direction | Meaning |
|---|---|---|
| `Mentions` | mention site → Entity | The source node textually mentions the target Entity. |
| `SameAs` | Entity ↔ Entity | Two Entity nodes refer to the same thing. Used sparingly; prefer one Entity per concept when possible. |
| `Refers` | reference text → resolved node | A textual reference ("see §5") resolves to a target node. Distinct from `Mentions` in that the source span carries explicit reference syntax. |

### 3. Rule Modification — how rules change other rules

| Relation | Direction | Meaning |
|---|---|---|
| `Excepts` | Exception → Rule | Source carves out cases the target rule does not apply to. |
| `Refines` | refined rule → parent rule | Source is a narrower version of target (sub-rule sharpens parent). |
| `Generalizes` | general rule → specific rule | Source is the broader version that target specializes. (Inverse of `Refines`; both directions provided for clarity in different elicitation flows.) |
| `Supersedes` | newer → older | Source replaces target; engine ignores target unless time-of-record predates source's effective date. |
| `Restricts` | constraint → rule | Source limits the scope of target without fully replacing it. |

### 4. Application — what a rule applies to

| Relation | Direction | Meaning |
|---|---|---|
| `AppliesTo` | Rule → Entity \| Fact | Target is the subject the rule speaks about. |
| `AppliesWhen` | Rule → Condition | Target is a condition that must hold for the rule to fire. |
| `Concludes` | Rule → Fact \| Atom | Target is the Fact the rule produces when fired. |

### 5. Provenance — where a node came from

| Relation | Direction | Meaning |
|---|---|---|
| `DerivedFrom` | derived → premise | Source is the conclusion of applying a Rule to target premises. Used during engine execution. |
| `JustifiedBy` | answer → chain | Target is one Rule (or sub-answer) in the justification chain for source. Multiple `JustifiedBy` edges form the proof DAG. |
| `ElicitedFrom` | Rule → LLM call | Target identifies the LLM call (call_record id) that produced this rule during ADJ14 elicitation. |

### 6. Tabular — structured table membership

| Relation | Direction | Meaning |
|---|---|---|
| `RowOf` | Cell → Row | Target Row contains source Cell. |
| `ColumnOf` | Cell → Column | Target Column contains source Cell. |
| `HeaderOf` | Header → Column \| Row | Source is the labeled header for target axis. |
| `CellOf` | Cell → Table | Source Cell belongs to target Table. Implied by `RowOf` + `ColumnOf` but kept for direct table-level navigation. |

### 7. Temporal — time-ordered relationships

| Relation | Direction | Meaning |
|---|---|---|
| `Before` | earlier event → later event | Source temporally precedes target. |
| `After` | later → earlier | Convenience inverse. |
| `During` | concurrent ↔ concurrent | Source coincides with target. |
| `EffectiveAt` | Rule → Date | Target Date (an Entity with `term: date(Y, M, D)`) is when source becomes effective. |
| `SupersededAt` | Rule → Date | Target Date is when source was retired. |

### 8. Cross-source — disagreements and confirmations across documents

| Relation | Direction | Meaning |
|---|---|---|
| `ConflictsWith` | Rule ↔ Rule | Source and target produce contradictory conclusions on overlapping inputs. ADJ09 §"Conflicts Between Sources". |
| `Confirms` | Rule → Rule | Source independently asserts what target asserts. Useful for cross-source corroboration. |
| `DependsOn` | Rule \| Rulebook → Rule \| Rulebook | Source requires target to be present and consistent. |

### 9. Discourse — linguistic relationships within text

| Relation | Direction | Meaning |
|---|---|---|
| `Defines` | Definition → Entity | Source provides the definition that names target. |
| `Restates` | Restatement → original | Source paraphrases target without adding content. Often used inside text as redundant emphasis. |
| `Cites` | source → authority | Source cites target as the authority (an Entity with `term: citation("49 CFR § 1540.111(a)")`). |

### 10. Refinement — clarification-driven node lineage

| Relation | Direction | Meaning |
|---|---|---|
| `Clarifies` | clarified node → original | Source replaces target after an ADJ06 clarification turn. Plays the same role as v2's `lowered_from`. |

Kind compatibility along `Clarifies` edges (unchanged from v2's
refinement-DAG rules):

```text
Fact         ←Clarifies←  Fact         (same claim, more specific)
Fact         ←Clarifies←  Uncertainty  (clarification resolved uncertainty)
Uncertainty  ←Clarifies←  Uncertainty  (more specific uncertain claim)
Query        ←Clarifies←  Query        (decomposition into subqueries)
Rule         ←Clarifies←  Rule         (rule refinement during compilation)
```

A `Fact` may **not** be `Clarifies`-clarified into an `Uncertainty` —
the framework treats a clarification that introduces uncertainty as a
structural failure, requiring an explicit Uncertainty node at the
original position.

### 11. Escape Hatch

| Relation | Direction | Meaning |
|---|---|---|
| `DomainSpecific(name)` | source → target | Deployment-specific relation. Source must be a string outside the names of the closed-set relations above. Validators record it but do not interpret it. |

A future v4 will promote frequently-used `DomainSpecific(name)`
relations into the closed set after they prove out across deployments.

## Polarity and Modality

```text
Polarity := Affirmed | Denied | Uncertain
Modality := Present | Past | Future | Hypothetical
          | FamilyHistory | RuledOut | Conditional
```

Unchanged from v2. Both flat lattices; no `Unknown` value for polarity
(the absence of evidence is the absence of a node, enforced by the
coverage check).

A new value `Inherit` is admitted on nodes and edges to mean "use the
effective value propagated from the structural parent". See
*Propagation* below.

## Node Kinds and Their Type Rules

### Fact

A claim about the world.

```text
kind = Fact implies:
    polarity      != Uncertain
    source_spans  is non-empty
    discard_reason is absent
```

### Query

A question the adjudication is asked.

```text
kind = Query implies:
    polarity      = Affirmed
    source_spans  may be empty (synthesized queries are allowed)
    discard_reason is absent
```

A Query may have empty source_spans if synthesized by the framework
(not extracted from text). All other kinds require non-empty spans.

### Uncertainty

The source explicitly raised a question without answering it.

```text
kind = Uncertainty implies:
    polarity      = Uncertain
    source_spans  is non-empty
    discard_reason is absent
```

### Rule

Produced by the rule-compilation pipeline (`ADJ09`) or the rulebook
elicitation phase (`ADJ14`).

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

A carve-out attached to one or more Rules. v3 removes the
`metadata.applies_to` requirement; the attachment is now expressed as
one or more `Excepts` edges (source = Exception, target = Rule).

```text
kind = Exception implies:
    polarity      = Affirmed
    source_spans  is non-empty
    discard_reason is absent
```

Every Exception node MUST be the source of at least one `Excepts`
edge. The validator enforces this.

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

### Section (new in v3)

A structural unit of the source document — paragraph, sentence,
subsection, list item, table, row, cell. Sections carry the *meta-text*
of the unit (heading, numbering, delimiter), not the content. The
content lives in child nodes connected by `Contains` edges.

```text
kind = Section implies:
    source_spans  is non-empty
    discard_reason is absent
    term          is a compound describing the section type:
                      section(level)       -- level = 1, 2, 3, ...
                      paragraph(_)
                      sentence(_)
                      list_item(index)
                      table(_)
                      row(index)
                      column(index)
                      cell(row, col)
                      heading(level)
```

A Section's source_spans cover only the structural markers (the "§1.",
"(a)", "|"). The semantically-meaningful content is reached by
following `Contains` edges to child nodes. A Section that has no
`Contains` edges out of it is structurally empty and triggers an
ADJ02 coverage violation.

### Entity (new in v3)

A deduplicated reference target for an atom or compound term that
appears (or could appear) at multiple sites in the document.

```text
kind = Entity implies:
    source_spans  may be empty (synthesized entities allowed)
    discard_reason is absent
    polarity      = Affirmed
    modality      = Present
    term          is the atom or compound the Entity stands for
```

Entities exist to factor common atoms out of repeated terms. A rule
that mentions `passenger` five times produces one Entity node
`E_passenger` with `term: atom("passenger")` and five `Mentions` edges
from each Rule/Fact to `E_passenger`. The `Mentions` edge's
source_spans cover the byte range where "passenger" was mentioned in
the rule's text.

When the same atom appears in only one place, an Entity node is
*optional* — the atom can stay inline in the mentioning node's term.
Validators don't require deduplication; the extractor LLM emits
Entities when it judges them useful.

## Polarity / Modality on Edges

Edges carry their own polarity and modality. The semantics:

- An edge with polarity `Denied` is **an explicit negation of the
  relation**, not a relation involving negated facts. `Excepts(E, R,
  polarity=Denied)` means "E does NOT except R", which an audit-
  trail-aware reader may want to record (e.g., "the standard exception
  does not apply here").
- An edge with modality `Conditional` is **a conditional assertion of
  the relation**. The condition itself is expressed in another edge
  (typically `AppliesWhen`) whose target is the condition.

The default for every edge is `polarity: Affirmed, modality: Present`.

## Propagation

Polarity and modality propagate **along `Contains` edges**. A child
node whose polarity (or modality) is `Inherit` adopts the effective
value of its `Contains` parent.

Rules:

1. If a node carries an explicit (non-`Inherit`) polarity, that value
   is its effective polarity.
2. Otherwise, follow `Contains` edges in reverse to the nearest
   ancestor with a non-`Inherit` polarity; that ancestor's effective
   polarity is also this node's effective polarity.
3. Modality propagates by the same rule.
4. **Multi-parent**: if a node has multiple `Contains` parents and
   `polarity = Inherit`, all parents' effective polarities must agree.
   Disagreement is a `PropagationConflict` validation error.

No other edge relation propagates polarity or modality. `Excepts`,
`Cites`, `RowOf` etc. carry their own.

`ADJ03-v3` formalises propagation and the consistency check.

## The Coverage Invariant (ADJ02 v3)

```text
union of (source_spans across all nodes)
  ∪ union of (source_spans across all edges)
  = [0, len(document))
```

with the exact conditions:

1. **No gaps.** Every byte from 0 to `len(document)` appears in at
   least one source_span.
2. **No overlaps.** Every byte appears in **at most one** source_span,
   summed across nodes and edges.
3. **Exceptions for synthesized objects.** Query nodes with empty
   spans are exempt; Entity nodes with empty spans are exempt;
   synthesized edges with empty spans are exempt.

This is a **flat tiling** check — no hierarchical recursion. The graph
structure is irrelevant to coverage; only the spans matter. A Section
node tiles its heading bytes, its children tile its content bytes,
edges between them tile the connectives ("and", "or", ",", "see").
Every byte is somewhere. Nothing is double-counted.

## The Acyclicity Invariant (ADJ02 v3 part 2)

The directed graph `G = (Nodes, Edges)` formed by treating every
edge as a directed link from `source` to `target` MUST be acyclic.

That is: there is no sequence of nodes `n_1, n_2, ..., n_k` (k ≥ 1)
such that there exists an edge `(n_i, n_{i+1})` for each `i` and
`n_k = n_1`.

The check applies **across all edge relations**. v3 forbids cycles
of any kind:

- No mutual references (`References` cycle).
- No mutual exception (`Excepts` cycle).
- No mutual refinement.
- No mutual supersession.

A future version may relax this for specific relations where cycles
are semantically meaningful (a `Refers` cycle in a document with
mutual cross-references, for instance). For now, the rule is uniform.

Detection: standard DFS with greys/blacks, O(|V| + |E|). When a cycle
is found, the validator returns the cycle's edges as the violation,
and ADJ06 can prompt the LLM to break the cycle (re-extract a
specific edge with a different target, or drop a redundant edge).

## Well-Formedness Summary

An IR document is well-formed iff:

1. **Node-kind constraints** — every node satisfies the rules for its
   kind (above).
2. **Edge well-formedness** — `source` and `target` both exist; the
   relation is from the closed set or is `DomainSpecific(name)` with a
   non-empty unique name.
3. **Identifier uniqueness** — every node id is unique among nodes;
   every edge id is unique among edges.
4. **Span validity** — every Span's offsets are valid byte ranges in
   the document.
5. **Coverage** — the union of all node and edge source_spans tiles
   `[0, len(document))` with no gaps and no overlaps, modulo the
   synthesized-object exemption.
6. **Acyclicity** — the graph `(Nodes, Edges)` is acyclic.
7. **Propagation consistency** — for every node with
   `polarity = Inherit`, all `Contains` parents agree on effective
   polarity (and similarly for modality).
8. **Edge-relation constraints** — relation-specific invariants hold:
   `Excepts` edges connect Exception → Rule; `RowOf` connects to a
   Section with `term: row(_)`; `Clarifies` edges respect kind
   compatibility; etc. See *Relation-Specific Invariants* below.
9. **Exception attachment** — every Exception node is the source of
   at least one `Excepts` edge.

`validate()` enforces all nine conditions and returns the specific
violation when any fails. Validation is total — no partial well-
formedness.

## Relation-Specific Invariants

Each relation imposes additional invariants on the kinds of its
endpoints. Validator-enforced:

| Relation | Source kind | Target kind |
|---|---|---|
| `Contains` | `Section` | any |
| `Precedes` | `Section` | `Section` |
| `Heading` | `Section` (with `term: heading(_)`) | `Section` |
| `Mentions` | any non-Entity | `Entity` |
| `SameAs` | `Entity` | `Entity` |
| `Refers` | any | any |
| `Excepts` | `Exception` | `Rule` |
| `Refines` | `Rule` | `Rule` |
| `Generalizes` | `Rule` | `Rule` |
| `Supersedes` | `Rule` | `Rule` |
| `Restricts` | `Rule` | `Rule` |
| `AppliesTo` | `Rule` | `Entity` \| `Fact` |
| `AppliesWhen` | `Rule` | any (the condition node) |
| `Concludes` | `Rule` | `Fact` \| `Entity` |
| `DerivedFrom` | `Fact` | `Fact` |
| `JustifiedBy` | `Fact` \| `Query` | `Rule` \| `Fact` |
| `ElicitedFrom` | `Rule` | `Entity` (with `term: call_record(_)`) |
| `RowOf` | any | `Section` (`term: row(_)`) |
| `ColumnOf` | any | `Section` (`term: column(_)`) |
| `HeaderOf` | `Section` (`term: heading(_)`) | `Section` |
| `CellOf` | any | `Section` (`term: table(_)`) |
| `Before` / `After` / `During` | any | any |
| `EffectiveAt` / `SupersededAt` | `Rule` | `Entity` (`term: date(_, _, _)`) |
| `ConflictsWith` | `Rule` | `Rule` |
| `Confirms` | `Rule` | `Rule` |
| `DependsOn` | `Rule` \| `Entity` | `Rule` \| `Entity` |
| `Defines` | any | `Entity` |
| `Restates` | any | any (same kind preferred) |
| `Cites` | any | `Entity` (`term: citation(_)`) |
| `Clarifies` | any | any (subject to kind-compatibility table above) |
| `DomainSpecific(name)` | any | any |

## Worked Example (Graph Form)

Continuing the TSA example, now extended to include a citation and a
rule reference. Source:

```text
§1540.111(a) Carry-on limits. A passenger may carry one (1) carry-on
bag plus matches, except a passenger over age 16 may not carry
strike-anywhere matches.
                                  (185 bytes)
```

The v3 IR:

```text
Nodes:
  N1  Section term=section(1)            spans=[0, 12]      # "§1540.111(a) "
  N2  Section term=heading(2)            spans=[12, 28]     # "Carry-on limits."
  N3  Rule    term=definitional(...)     spans=[29, 96]     # "A passenger may carry one (1) carry-on bag plus matches"
  N4  Exception                          spans=[97, 168]    # "except a passenger over age 16 may not carry strike-anywhere matches"
  N5  Entity  term=atom(passenger)       spans=[]           # deduplicated
  N6  Entity  term=atom(matches)         spans=[]
  N7  Entity  term=atom(strike_anywhere) spans=[]
  N8  Entity  term=citation("49 CFR § 1540.111(a)") spans=[]

Edges:
  E1  N2  --Heading--> N1               spans=[]                # heading-of-section
  E2  N3  --Mentions--> N5              spans=[31, 40]          # "passenger" in rule text
  E3  N3  --Mentions--> N6              spans=[78, 85]          # "matches" in rule text
  E4  N3  --Cites    --> N8             spans=[]                # rule cites the regulation
  E5  N4  --Excepts  --> N3              spans=[97, 103]         # "except"
  E6  N4  --Mentions --> N5              spans=[104, 113]        # "passenger" in exception text
  E7  N4  --Mentions --> N7              spans=[151, 167]        # "strike-anywhere"
  E8  N4  --Mentions --> N6              spans=[168, 175]        # "matches"
  E9  N3  --AppliesTo--> N5              spans=[]                # synthesized; passenger is the subject
  E10 N1  --Contains --> N2              spans=[]
  E11 N1  --Contains --> N3              spans=[]
  E12 N1  --Contains --> N4              spans=[]
  E13 ... (Contains edges to a Discarded for the trailing "." if any)
```

(Spans above are illustrative byte ranges; precise offsets depend on
exact source bytes.)

**Coverage check**: union of all source_spans across nodes and edges
tiles `[0, 185]` exactly once. The Section spans cover the structural
markers. The Rule and Exception spans cover the rule and exception
text. The Mentions and Excepts edges cover the connectives and entity
mentions. Synthesized objects (Entity nodes with empty spans, the
AppliesTo edge, Contains edges) don't tile.

**Acyclicity check**: `Heading`, `Contains`, `Mentions`, `Cites`,
`Excepts`, `AppliesTo` all point in compatible directions. No cycle
exists. The DAG validates.

**Polarity propagation**: every node carries an explicit polarity;
no `Inherit` resolution needed in this example.

Compare with v2's tree-only representation, which had no slot for the
citation (N8 / E4) and inlined the exception's "applies-to" target via
metadata rather than as an `Excepts` edge. The v3 graph makes both
explicit and audit-trail-recordable.

## Serialization

The on-disk and on-wire form is JSON. A JSON Schema for `IRDocument`
will live at `code/specs/schemas/adjudication-ir.schema.json` (added
when the v3 Rust crate lands).

Top-level shape:

```json
{
  "document_id": "...",
  "nodes": [ /* IRNode objects */ ],
  "edges": [ /* IREdge objects */ ]
}
```

Schema changes from v2:

- `kind` enum loses `"TextRun"`, gains `"Section"` and `"Entity"`.
- `part_of` and `lowered_from` fields removed from `IRNode`.
- New top-level `edges` array.
- `polarity` and `modality` admit `"Inherit"` on both nodes and edges.

## Lowering to the Engine

The connector (`adjudication-connector`) lowers an IR document to a
Prolog/ProbLog program by emitting:

- One fact (or rule clause) per `Rule` node.
- One fact per `Fact` node.
- One Prolog atom per `Entity` node.
- One Prolog clause per typed edge: `excepts(E, R)`, `applies_to(R,
  X)`, `cites(R, A)`, `row_of(C, R)`, etc. The engine reasons over
  these clauses natively.

This means the engine can now answer queries like "list every rule
that cites authority A" — a thing it could not do over v2's IR because
citations weren't represented.

The connector spec (`ADJ11` and follow-ups) handles the lowering
details. ADJ01 v3 only defines the IR shape the connector consumes.

## Open Questions (v3)

1. **Entity dedup threshold.** Should an atom that appears twice
   become an Entity, or only when it appears `N ≥ 3` times? The
   tradeoff is graph size vs. small-model burden (every Entity is one
   more node the LLM must emit). The current design leaves the
   decision to the extractor; benchmarking under ADJ12 should inform
   guidance.
2. **Multi-document IR.** An adjudication that involves a clinical
   note plus a formulary plus a state regulation has three documents
   with cross-document edges (`Cites` from note to formulary, etc.).
   The DocumentId field already supports it; the validator and engine
   handle a "session" of multiple IRs. Wire-format conventions for
   cross-document edges are open.
3. **Polarity propagation along other edges.** Today only `Contains`
   propagates. Should `Refines` propagate too (a refined rule
   inheriting the parent's polarity)? Probably; the current spec is
   conservative.
4. **Edge merging.** If two `Mentions` edges from N1 to N5 differ only
   in source_spans, should they merge into one edge with the union of
   spans? The current spec keeps them separate (each mention is its
   own edge). Merging could reduce graph size at the cost of losing
   distinct mention-site provenance.
5. **`Inherit` on edges.** Currently `Inherit` is only meaningful on
   nodes (propagation along `Contains`). An `Inherit`-polarity edge
   has no well-defined meaning. The spec admits the value
   syntactically but the validator rejects it on edges. This may be
   relaxed once a use case appears.

## Limitations (v3)

1. **The LLM has more to emit.** v3 asks the extractor to produce
   edges in addition to nodes. Small models will struggle initially.
   The decompose_text prompt (next bump → `decompose-text-v4`) will
   need worked examples for edges, and the ADJ06 retry primitives will
   handle the most common LLM failure modes (missing edge, miscategorized
   relation, dangling endpoint). The framework's intelligence-in-the-
   pipeline thesis still applies — the LLM doesn't have to get it right
   the first time, but the audit trail will show every attempt.
2. **The validator is bigger.** The well-formedness check is more
   expensive (DAG check is `O(V + E)`, coverage is `O(N log N)` over
   sorted spans). For long documents (>1 MB of source), profiling may
   suggest worker-thread parallelism or streaming validation. Out of
   scope for v3.
3. **Graph queries at answer time.** Users may ask "show me every rule
   that cites authority A and supersedes a 2023 rule". Such queries
   compose engine lowering with graph traversal. The connector + the
   engine handle it, but expressing the query is a UX concern, not an
   ADJ01 concern.
4. **No cycles for now.** A `Refers` cycle (mutual cross-references)
   is sometimes legitimate; v3 forbids it uniformly. Relaxation is
   planned for specific relations after deployment experience informs
   which cycles are real vs. error.

## Status

v3 draft. The graph IR replaces the v2 hierarchical tree across
`ADJ02`, `ADJ03`, `ADJ04`, `ADJ05`, the `adjudication-ir` Rust crate,
the four checker crates, the connector, and the `decompose_text`
primitive's prompt. Each of those updates lands in a follow-up PR
sequenced after this spec. v2 leaves no shipped artefacts; the
migration cost is entirely in the framework code, not in any deployed
adjudication or persisted IR.

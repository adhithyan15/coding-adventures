# ADJ02 — Coverage Checker: Structural Tree Check Over the Hierarchical IR

> **Pending revision to v3 (2026-05-12)** to match
> [`ADJ01 v3`](ADJ01-adjudication-ir-grammar.md): coverage becomes a
> *flat tiling* check over the union of node and edge `source_spans`
> (no recursive tree descent), and a separate **DAG acyclicity**
> invariant is added. The v2 content below describes the previous
> structural-tree check and is preserved until the v3 revision lands
> alongside the `adjudication-coverage` crate's v3 update.

> **Revision v2 (2026-05-11): structural coverage.** v1 of this spec
> defined coverage as a token-level check driven by a language-
> specific tagger. That approach baked English-language assumptions
> into the framework core. This revision replaces the tagger-based
> path with a **structural tree check** over the hierarchical IR
> introduced in [`ADJ01` v2](ADJ01-adjudication-ir-grammar.md). The
> check is language-agnostic by construction, deterministic, linear
> in IR node count, and requires no stopword lists or tokenizers.

## Overview

The coverage invariant in v2 is:

> Every byte of the document's normalized text is in the source
> spans of some leaf in the IR's decomposition tree.

Equivalently: every `TextRun` parent's children's `source_spans`
union to the parent's `source_spans`, and the root's `source_spans`
cover the document. By induction, every byte ends up in some leaf.

There is no language-specific knowledge in this check. There is no
tagger. There is no stopword list. The LLM that produced the IR
chose how to decompose the document; the framework verifies the
decomposition is complete.

## What the Check Catches

| v1 (rule-based) caught | v2 (structural) catches |
|---|---|
| English tokens not covered by any IR node | Any byte of any document, in any language, not transitively covered by a leaf |
| Silent extraction omission (the canonical case) | Same — same failure mode, language-agnostic check |
| `Unparseable` Discarded as hard failure | Same — `Unparseable` remains a hard failure |

The class of failure caught is the same: silent omission. What
changes is the *mechanism* of detection. v1 needed to know which
tokens were "meaningful." v2 needs only the IR's structural tree —
which the LLM produced — and verifies the tree tiles the document.

## Layer Position

```
   ADJ01 v2 hierarchical IR
        │
        ▼
   ADJ02 v2 structural coverage    ← this document
        │
        ▼
   ADJ03 v2 propagation consistency
        │
        ▼
   ADJ04 round-trip entailment
        │
        ▼
   ADJ05 adversarial verifier
```

## Inputs and Outputs

The coverage check takes:

- A `Document` carrying `(DocumentId, normalized_text)`. The check
  reads only the *length* of `normalized_text` (the byte count); it
  never inspects the bytes.
- An `IRDocument`: a list of `IRNode`s with `part_of` edges forming
  a forest of trees.

It returns either:

- **`Pass`** — every byte is covered.
- **`Fail(violations)`** — one or more structural-coverage
  violations.

The check is **deterministic and language-free**. It does not call
an LLM at check time.

## The Five Structural Conditions

For an IR document to satisfy coverage:

1. **Span-validity**: every `Span.start < Span.end`, both within
   `[0, len(normalized_text)]` for the document's id.
2. **Root coverage**: the union of the roots' `source_spans` (roots
   = nodes with `part_of = None`) equals `[(doc, 0, len)]`. The
   roots collectively tile the whole document.
3. **Parent-child containment**: for every node `X` with
   `X.part_of = Some(Y)`, `X.source_spans ⊆ Y.source_spans`.
4. **TextRun tiling**: for every `TextRun` `Y`, the union of `Y`'s
   children's `source_spans` equals `Y.source_spans`. Every byte of
   the parent's spans is covered by some child.
5. **No `Unparseable`**: any `Discarded` node with `discard_reason
   = Unparseable` is a hard coverage failure (per ADJ01).

These five conditions together imply the headline invariant.
Verification is linear in the number of nodes plus the total length
of source-span lists.

## The Algorithm

```text
coverage_check(doc, ir_doc) -> Result:
    violations = []

    # 1. Span validity (constant per span)
    for node in ir_doc.nodes:
        for span in node.source_spans:
            if span.document_id != doc.id:
                violations.push(SpanWrongDocument { node.id, span })
                continue
            if span.start >= span.end:
                violations.push(InvalidSpan { node.id, span })
            elif span.end > len(doc.normalized_text):
                violations.push(SpanOutOfBounds { node.id, span })

    # 2. Unparseable hard failure
    for node in ir_doc.nodes:
        if node.kind == Discarded and node.discard_reason == Unparseable:
            violations.push(UnparseableDiscarded { node.id })

    # 3+4. Build child lists per parent, including roots.
    children_of = collect_children(ir_doc.nodes)
    roots       = [n for n in ir_doc.nodes if n.part_of is None]

    # 5. Root tiling: union(roots' spans) == doc's full range.
    if not spans_equal(union(roots.map(.source_spans)),
                       [(doc.id, 0, len(doc.normalized_text))]):
        violations.push(RootsDoNotTileDocument)

    # 6. For each non-root, parent-child containment.
    for node in ir_doc.nodes:
        if node.part_of is None: continue
        parent = ir_doc.find(node.part_of)
        if parent is None:
            violations.push(DanglingPartOf { node.id })
            continue
        if not is_subset(node.source_spans, parent.source_spans):
            violations.push(ChildSpansExceedParent { node.id, parent.id })

    # 7. For each TextRun, children tile parent.
    for parent in ir_doc.nodes:
        if parent.kind != TextRun: continue
        kids = children_of.get(parent.id, [])
        if not spans_equal(union(kids.map(.source_spans)),
                           parent.source_spans):
            violations.push(ChildrenDoNotTileParent { parent.id })

    return Pass if violations is empty else Fail(violations)
```

`union`, `is_subset`, and `spans_equal` operate on lists of byte
ranges, sorted and merged in `O(n log n)` per call. The overall
check is `O(N log N)` where `N` is the total span count.

## Violation Types

```text
CoverageViolation :=
    SpanWrongDocument { node_id, span }
  | InvalidSpan { node_id, span }
  | SpanOutOfBounds { node_id, span }
  | UnparseableDiscarded { node_id }
  | RootsDoNotTileDocument { missing_ranges }
  | DanglingPartOf { node_id, missing_parent }
  | ChildSpansExceedParent { child_id, parent_id }
  | ChildrenDoNotTileParent { parent_id, missing_ranges }
```

Each variant carries enough information for `ADJ06` to render a
clarification question. `missing_ranges` lists the specific byte
ranges that were not covered, so the framework can surface those
specific portions of the input back to the extractor or the user.

## Clarification Generation

When the check fails, the violation types map to question shapes:

| Violation | Clarification question shape |
|---|---|
| `RootsDoNotTileDocument` or `ChildrenDoNotTileParent` | *"You did not account for this portion of the input: \<text of missing range\>. What does it mean?"* |
| `ChildSpansExceedParent` | *"This claim's spans extend beyond its parent context — please re-examine."* (re-prompt the LLM rather than the user) |
| `UnparseableDiscarded` | *"The span '\<text\>' could not be interpreted. Could you rephrase it, or confirm it can be discarded with a specific reason?"* |
| `DanglingPartOf` or `InvalidSpan` | Re-prompt the LLM with the malformed-IR detail. |

Per `ADJ06`, the cheapest rung (re-prompt the extractor) handles the
malformed-IR cases. Genuine missing-content cases escalate via the
ladder.

## Strictness — Reduced from v1

v1 specified `Strict` / `Permissive` / `AuditOnly` strictness modes
because the rule-based tagger could classify ambiguous tokens. v2's
check has nothing to be lenient about — either every byte is in
some leaf or it isn't. The strictness configuration is removed.

`AuditOnly` is replaced by a `report_only: bool` field in the
caller's configuration: when true, violations are reported in
telemetry but do not gate the adjudication. The check itself is
unchanged.

## Comparison to v1

| Aspect | v1 (rule-based) | v2 (structural) |
|---|---|---|
| Tagger / stopword list | Required, English-specific | Removed |
| NegEx / ConText triggers | Required by ADJ03 | Removed (moved to LLM) |
| Languages supported | English (effectively) | Any language the LLM handles |
| Algorithm | Token classification + interval cover | Tree-shape verification |
| Asymptotic complexity | `O(tokens × IR nodes)` | `O(IR nodes log N)` |
| Determinism | Yes (rule-based) | Yes (structural) |
| LLM calls at check time | Zero | Zero |
| Failure modes caught | Silent omission via untagged tokens | Silent omission via gaps in the tree |

The check is **strictly cheaper** in v2 (no token classification) and
**strictly more general** (no English assumption).

## Worked Example (Revised)

Continuing the TSA case. The LLM produces this decomposition:

```text
N0 TextRun                     spans = [(doc, 0, 209)]
   ├── F1..F5  (Facts: toothpaste, perfume, batteries, wine, knife)
   │           spans tile (0, 149)
   ├── N2 TextRun polarity=Denied   spans = [(doc, 150, 176)]
   │     └── F6 Fact carry_on_item(matches)  spans = [(doc, 150, 176)]
   └── F7  Fact carry_on_item(lighter, ...)  spans = [(doc, 177, 209)]
```

Coverage check:

- Span validity: every span is in `[0, 209]` and `start < end`. **Pass**.
- No `Unparseable`. **Pass**.
- Roots: N0 has spans `[(doc, 0, 209)]`; that equals the document's
  full range. **Pass**.
- Parent-child containment: every child's spans are inside N0's
  (and N2's children inside N2's). **Pass**.
- TextRun tiling:
  - N0's children's spans are F1..F5 (0..149) + N2 (150..176) +
    F7 (177..209). Union = `[(doc, 0, 209)]` = N0's spans. **Pass**.
  - N2's children are just F6 with `(150, 176)`. Union = N2's spans.
    **Pass**.

Result: **Pass**. No bytes uncovered; no `Unparseable`; tree well-
formed.

Now the counterexample. The LLM emits the same decomposition but
forgets to include F7:

```text
N0 TextRun     spans = [(doc, 0, 209)]
   ├── F1..F5  spans tile (0, 149)
   └── N2 TextRun spans = [(doc, 150, 176)]
         └── F6 spans = [(doc, 150, 176)]
```

Now N0's children's spans union to `(0, 149) ∪ (150, 176) = (0, 176)`.
But N0's spans are `(0, 209)`. The tiling check fails with
`ChildrenDoNotTileParent { parent_id: N0, missing_ranges: [(177, 209)] }`.

Clarification fires: *"You did not account for this portion of the
input: 'only a single disposable lighter.'. What does it mean?"*
The user (or the extractor on re-prompt) responds; F7 is added; the
check re-runs and passes.

## Open Questions

1. **Empty TextRuns.** A TextRun with zero children is structurally
   ill-formed (it doesn't tile its own spans). Currently the check
   reports `ChildrenDoNotTileParent { parent_id, missing_ranges =
   parent.source_spans }`. This is the right diagnostic but a
   `TextRunHasNoChildren` variant would be more specific. Open.
2. **Overlapping siblings.** Two children may have overlapping spans
   if a phrase is genuinely double-classified. Currently `spans_equal`
   uses set equality of merged intervals, so overlap doesn't break
   the tile check. Whether the framework should *flag* overlap as a
   warning is open.
3. **Documents with whitespace-only trailing content.** A document
   ending in `"\n\n\n"` may yield a tree where the LLM didn't
   produce a leaf for the trailing newlines. Currently the check
   reports those bytes as uncovered. The pragmatic fix is for the
   document normalizer to trim trailing whitespace before the IR is
   built; the framework does not normalize at check time.

## Limitations

1. **The check is structural, not semantic.** A TextRun that
   correctly tiles the document but groups unrelated content
   together (one paragraph spanning two unrelated topics) is
   accepted. Semantic grouping quality is verified by `ADJ04`
   (round-trip) and `ADJ05` (adversarial), not here.
2. **The check trusts the LLM's decomposition.** The framework
   verifies tree shape, not that the LLM made the *right* tree.
   Different LLMs may produce different valid decompositions of the
   same document. Reproducibility relies on the audit-trail
   discipline (versioned models, prompts, seeds).

## Status

v2 draft. Replaces the v1 tagger-based path. Implementation depends
on the `ADJ01` v2 grammar; the Rust crate `adjudication-coverage` is
being rewritten in parallel (the v1 `RuleBasedTagger` is retired).

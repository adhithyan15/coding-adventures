# ADJ33 — Partial-IR Instrumentation: The Dominant Failure Mode Is `NoChildrenAtLevel`

> Instrumentation + diagnostic data PR. ADJ30 and ADJ32 both
> falsified intervention hypotheses without data on the raw model
> output. ADJ33 extends `HierarchicalDecomposeError::CoverageUnresolved`
> with a `partial_ir` field and the bench binary with `gaps_detail`
> + `partial_ir` JSON, then re-runs the 6 cells we have been
> studying.
>
> **Finding: the dominant failure mode is not what any prior
> hypothesis assumed.** 6 of 11 residual gaps across the 5
> parseable cells are `NoChildrenAtLevel` — the model is given a
> parent text (e.g., the Phrase `"1 carry-on bag, "`) and returns
> *zero* children. Not truncated children, not wrongly-spanned
> children: an empty `nodes: []` array.
>
> Three other failure shapes appear: `UncoveredBytes` (3 cases —
> the model skipped digits/spaces/commas it apparently treats as
> non-content), `ChildSpansEscape` (1 case — a child node spans
> bytes outside its declared parent), `FlattenedAtom` (1 case —
> the atom name `"bag_count"` smuggles quantity into the term).
>
> The "trailing punctuation" / "retry budget" / "prompt length"
> interventions are all *wrong-target*. The right intervention is
> **a contract for what to emit when no claim is recoverable** —
> probably a per-level fallback rule the orchestrator can apply
> deterministically rather than waiting for the model to learn
> the right behavior.
>
> Raw data: [`data/adj33-partial-ir-bench-2026-06-01.json`](data/adj33-partial-ir-bench-2026-06-01.json).

## What ADJ33 ships

1. **`HierarchicalDecomposeError::CoverageUnresolved` carries
   `partial_ir`** (`IRDocument`). The orchestrator passes the
   IR-state-at-give-up so diagnostic tooling can see what the
   model actually produced.
2. **`adj_pr6_bench` emits two new fields** on
   `coverage_unresolved` errors:
   - `gaps_detail`: array of `{level, parent_node_id,
     kind_debug}` per gap.
   - `partial_ir`: compact JSON dump of the partial IR's nodes
     and edges (hand-rendered; `adjudication-ir` doesn't derive
     `Serialize`).
3. **6-cell empirical run** on the H1 cell set
   (matches × {0.5b, 1.5b, 3b} + lighter-disposable × same) at
   the ADJ29 default budget, with the new instrumentation.

## What the data shows — per-cell

### `matches × qwen2.5:0.5b` (3 gaps)

```
Source: "1 carry-on bag, matches."  (24 bytes)
Partial IR: 4 nodes (Doc, S1, P1, F1)
  - Doc span=[0,24)
  - S1=[11,24) "bag, matches."   ← model dropped "1 carry-on " (0-11)
  - P1=[11,24) "bag, matches."
  - F1=[11,24) "bag, matches."

Gaps:
  1. DocumentToSentence on Doc: UncoveredBytes [(0, 11)]
     → the model treated "1 carry-on " as non-source
  2. FactToTypedComponent on F1: NoChildrenAtLevel
     → the model emitted no TypedComponents for "bag, matches."
  3. PhraseToClaim on P1: FlattenedAtom
     atom="bag_count" with reason UnitSuffix{"_count"}
     → atom name embeds quantity instead of using a Quantity TypedComponent
```

### `matches × qwen2.5:1.5b` (1 gap)

```
Source: "1 carry-on bag, matches."  (24 bytes)
Partial IR: 6 nodes
  - P1=[0,16) "1 carry-on bag, "  ← correctly tiled

Gap:
  PhraseToClaim on P1: NoChildrenAtLevel
  → the model was given "1 carry-on bag, " and emitted zero Claim nodes
```

This is the cell whose g=1 ADJ32 tried (and failed) to close with
a trailing-punctuation worked example. The actual failure is
**not** trailing punctuation. The model produced the Phrase
correctly with the trailing comma+space; it then refused to emit
any Claim children for that Phrase.

### `matches × qwen2.5:3b` (1 gap)

```
Source: "1 carry-on bag, matches."  (24 bytes)
Partial IR: 1 node (Doc only)

Gap:
  DocumentToSentence on Doc: NoChildrenAtLevel
  → the model emitted ZERO Sentence children for the document
```

The 3B model failed at the very first level. Given a 24-byte
document, it produced no sentences. **At the very top of the
hierarchy, the model is willing to emit `nodes: []`.**

### `lighter-disposable × qwen2.5:0.5b` (5 gaps)

```
Source: "1 carry-on bag, disposable lighter."  (35 bytes)
Partial IR: 4 nodes
  - Doc=[0,35)
  - S1=[2,14) "carry-on bag"    ← skipped "1 " (0-2) and ", " (14-16)
  - S2=[16,35) "disposable lighter."
  - P1=[27,35) "lighter."        ← span ESCAPES its parent S1 ([2,14))

Gaps:
  1. DocumentToSentence on Doc: UncoveredBytes [(0,2), (14,16)]
     → "1 " and ", " not in any Sentence
  2. SentenceToPhrase on S1: ChildSpansEscape — P1 outside S1
  3. SentenceToPhrase on S1: UncoveredBytes [(2, 14)] — nothing covers "carry-on bag"
  4. SentenceToPhrase on S2: UncoveredBytes [(16, 27)] — "disposable " uncovered
  5. PhraseToClaim on P1: NoChildrenAtLevel
```

The 0.5B model is dropping digits, leading/trailing spaces, and
commas at the Sentence boundary (treating them as non-content),
and assigning Phrase children to the wrong parent Sentence
(ChildSpansEscape).

### `lighter-disposable × qwen2.5:1.5b` (1 gap)

```
Source: "1 carry-on bag, disposable lighter."  (35 bytes)
Partial IR: 7 nodes
  - P1=[0,16) "1 carry-on bag, "  ← correctly tiled

Gap:
  PhraseToClaim on P1: NoChildrenAtLevel
  → same as matches × 1.5b: model refuses to emit Claim children
    for "1 carry-on bag, "
```

Same failure mode as `matches × 1.5b`. Two cells, same model,
same Phrase shape, same `NoChildrenAtLevel` failure at exactly
the same boundary. This is now a *systematic 1.5B-class behavior*,
not noise.

### `lighter-disposable × qwen2.5:3b` (unparseable)

```
Source: "1 carry-on bag, disposable lighter."  (35 bytes)
Error: unparseable_at_FactToTypedComponent for parent F1
```

The 3B model produced an unparseable response at the deepest
level. (No partial IR available; `UnparseableResponse` carries
only the raw JSON the model emitted, not the IR built so far.)

## Tally — gap kinds across the 11 parseable gaps

| Gap kind | Count |
|---|---|
| **`NoChildrenAtLevel`** | **6** (55%) |
| `UncoveredBytes` | 3 (27%) |
| `ChildSpansEscape` | 1 (9%) |
| `FlattenedAtom` | 1 (9%) |

> **The dominant failure mode is the LLM returning an empty
> `nodes: []` array when asked to decompose a parent.** This is
> the single most consequential finding across ADJ30 / ADJ31 /
> ADJ32 / ADJ33 combined.

## Why this invalidates prior interventions

- **ADJ30 budget bump** (8 → 16 retries at FactToTypedComponent):
  if the model returns `nodes: []` on every attempt, more attempts
  do not help. The retry primitive submits the same parent text
  with the same prompt; the model emits the same empty array;
  the budget exhausts. **The failure mode is invariant under
  retry.**
- **ADJ32 prompt extension** (trailing-punctuation examples):
  the targeted Phrase `"1 carry-on bag, "` was *already* tiled
  correctly by the SentenceToPhrase step. The Phrase node
  contained the right text. The CLAIM_PROMPT was given that
  exact text and emitted `nodes: []`. **No amount of trailing-
  punctuation guidance fixes "emit no claims at all."**
- **ADJ29 per-level retry budget** (3/4/5/8): same as ADJ30 —
  retries on `nodes: []` get more `nodes: []`. Per-level budgets
  improve *some* failure modes (deeper-level fan-out cases) but
  not this one.

## What this points at for the real intervention

Two paths, both targeted at the actual failure mode:

### Path A — Prompt-level "always emit at least one child" rule

The current CLAIM_PROMPT says:

> "never mark the WHOLE input as Discarded. The phrase given to
> you contains real content; find the claim inside it. If unsure,
> prefer `is_fact: true` with a generic term."

This already tries to forbid the `Discarded`-the-whole-thing
escape hatch. But it doesn't forbid `nodes: []`. The model
appears to have found a *third* path: don't emit a Discarded
node, but also don't emit a Fact — emit nothing.

A targeted prompt addition would say:

> "EVERY response MUST contain at least one node. If you cannot
> identify the claim shape, default to `is_fact: true` with
> `term: {atom: "unknown"}` and the full phrase text. Returning
> an empty `nodes: []` array is a framework-level failure and
> the framework will reject it."

But ADJ32 demonstrated that **extending the CLAIM_PROMPT
regresses small-model output across every level**. The
prompt-length tax is real. So the fix has to be terse.

### Path B — Orchestrator-level deterministic fallback

When the model returns `nodes: []` for a parent, the orchestrator
can synthesize a fallback child:

```text
fallback_child = {
    id: <fresh>,
    kind: <level-appropriate default — Fact for PhraseToClaim,
           Sentence for DocumentToSentence, etc.>,
    text: <the full parent text>,
    term: {atom: "unknown"},
    polarity: Affirmed,
    modality: Present,
    /* the audit trail records this was synthesized */
}
```

This sidesteps the prompt entirely. The framework guarantees a
child exists; the model's empty-array response is treated as
"I have nothing to contribute, please use the default." The
audit trail records the synthesized node, so a reviewer can see
exactly which steps were model-derived and which were framework
defaults.

**Path B is structurally cleaner**: it doesn't depend on the
small model learning new behavior, doesn't add prompt-length
tax, and the audit trail is honest about which nodes are
synthesized. It also matches the framework's "intelligence in
the framework, not the model" thesis exactly: the model emits
nothing useful, so the framework emits the safe default.

ADJ34 (planned) implements Path B and re-benches.

## What this doesn't fix on its own

Path B addresses 6 of the 11 gaps (the `NoChildrenAtLevel`
cases). The other 5 — `UncoveredBytes`, `ChildSpansEscape`,
`FlattenedAtom` — need separate interventions:

- **`UncoveredBytes` at DocumentToSentence** (3 gaps on small
  models): the 0.5B model dropping digits/spaces/commas. A
  candidate fix is a SENTENCE_PROMPT addition saying "the input
  may start or end with whitespace, digits, or punctuation —
  these MUST be included in a node (typically as Discarded with
  reason DocumentMetadata)." But ADJ32 warns: prompt extensions
  on small models can backfire. Probably also wants a Path-B
  fallback at this level.
- **`ChildSpansEscape`** (1 gap, 0.5B): the orchestrator's
  splice could reject children whose spans escape their parent
  and trigger a synthesized fallback per Path B.
- **`FlattenedAtom`** (1 gap): the `bag_count` atom name embeds
  the quantity. The framework already detects this; the question
  is what to do beyond logging. A clear answer would be:
  decompose the flattened atom name into a Fact + Quantity
  TypedComponent automatically. This is an orchestrator change,
  not a prompt change.

## What's actually shipped here

1. **Code (`hierarchical.rs`)**: `CoverageUnresolved` variant
   grows a `partial_ir: IRDocument` field. The orchestrator
   passes the IR at the give-up point when returning this
   error. All match arms updated to use `{ gaps, .. }`.
2. **Code (`adj_pr6_bench.rs`)**: emits two new fields on
   `coverage_unresolved` errors — `gaps_detail` (per-gap
   `{level, parent_node_id, kind_debug}`) and `partial_ir`
   (compact node + edge dump).
3. **Spec (this doc)**: the diagnostic analysis above.
4. **Data**: 6-cell JSON output in
   `code/specs/data/adj33-partial-ir-bench-2026-06-01.json`.

The bench binary's existing JSON shape is **backwards-compatible**:
the new fields are `null` when the error variant isn't
`CoverageUnresolved`, and the old per-level distribution shape
is unchanged.

## Cost summary

| Metric | Value |
|---|---|
| Bench cells run | 6 |
| Wallclock total | ~5.5 min |
| Code added | ~70 LOC (orchestrator + bench binary) |
| Cells passed | 0 |
| Failure modes localized | 4 (NoChildrenAtLevel, UncoveredBytes, ChildSpansEscape, FlattenedAtom) |

## Gating condition — still NOT met, but the diagnosis is now precise

Zero cells fully passing. Tier 1 unblock requires 5/40. ADJ33
contributes:

- 0/6 cells closed
- One diagnostic instrumentation feature (`partial_ir` +
  `gaps_detail`) usable on all future benches
- **Concrete diagnosis**: 55% of residual gaps are
  `NoChildrenAtLevel`. The retry-and-prompt-extension
  interventions that ADJ30 and ADJ32 falsified were not just
  wrong — they were aiming at *the wrong failure mode entirely*.
- Two concrete paths forward (prompt rule vs. orchestrator
  fallback), with ADJ34 set up to implement Path B.

## See also

- [ADJ32](ADJ32-claim-prompt-trailing-punctuation-bench-results.md)
  — falsified the CLAIM_PROMPT trailing-punctuation extension,
  which set up the need for this diagnostic.
- [ADJ31](ADJ31-per-level-gap-distribution.md) — per-level gap
  distribution instrumentation (the predecessor to this one).
- [ADJ30](ADJ30-fact-typed-budget-bump-bench-results.md) —
  falsified the FactToTypedComponent budget bump.
- [ADJ29](ADJ29-per-level-retry-budget-bench-results.md) — the
  per-level retry budget bench.

## Status

- 2026-06-01: bench-binary instrumentation landed; orchestrator
  error variant extended; 6-cell run captured partial IR + gap
  detail; dominant failure mode identified as `NoChildrenAtLevel`.
- Next (ADJ34): implement Path B — orchestrator-level deterministic
  fallback when a parent's decomposition returns `nodes: []`.
  Re-bench. Hypothesis H6: the deterministic fallback closes
  the 6 `NoChildrenAtLevel` gaps without regressing any other
  metric.

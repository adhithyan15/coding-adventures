# HL07 — Spine Expansion to B1

## Status and purpose

This spec grows the shared can-do spine from a greeting-and-introductions prefix into
a complete pre-A1 → B1 course backbone, so that a "complete" book in any track has
somewhere to go.

It extends [HL04](./HL04-shared-spine-and-content-pipeline.md), which defined the
spine, and is a prerequisite for the content fan-out in
[HL05](./HL05-chapter-capability-and-step-by-step-shape.md).

## The gap this closes

The spine is the smallest load-bearing file in the curriculum, and it is far smaller
than its role implies:

| Observation | Value |
|---|---|
| Spine nodes | **11** |
| Concepts in the taxonomy | 46 |
| Nodes at stage pre-A1 | 7 |
| Nodes at stage A1 | 4 |
| Nodes at stage **A2** | **0** |
| Nodes at stage **B1** | **0** |
| Tracks whose authored chapters exceed the spine's reach | most — Latin has 36 chapters, Hindi 33, Spanish 34 |

`A2` and `B1` are declared in the spine's `stages` array with no nodes beneath them.
Eleven nodes carry the reader from *hello* to *counting to five*.

This produces a specific, visible failure: tracks have grown well past what the spine
describes, so their later chapters are held together by language-specific extension
nodes rather than by shared structure. The spine stops being the thing that makes
twenty tracks comparable. HL05's capability layer would inherit that weakness —
chapters beyond the spine's reach would have no shared can-do to realise.

**Eleven nodes cannot carry a complete book.** No amount of additional word lessons
fixes that; the backbone itself has to grow.

## Node design rules

A spine node keeps its existing six fields — `id`, `stage`, `canDo`, `prerequisites`,
`core`, `concepts` — and gains no new ones. Growth is in quantity and coverage, not
in schema.

1. **A node is a capability, not a topic.** `canDo` is one first-person sentence
   describing something the reader can do with the language. "I can say what I want
   and ask for it politely" is a node. "Verbs" is not.
2. **Ordering is by usefulness, not by grammar.** HL00's frequency-driven rule
   governs: what does a real basic conversation need next? A node earns its place by
   unlocking conversation, never by completing a paradigm.
3. **A node is language-independent.** Anything true only of one language belongs in
   that track's extension nodes, not in the spine. The existing `omits` / `relocates`
   ledgers absorb genuine mismatches.
4. **`core` marks the parity set** — nodes every track is expected to realise.
   Non-core nodes are legitimately skippable where a language does not mark the
   distinction, exactly as `SPINE-DEFINITE-REFERENCE` already is.
5. **Prerequisites stay shallow.** Deep chains reintroduce the gating this curriculum
   exists to avoid. A node should depend on what it genuinely needs and nothing more.

## Target shape

Growth toward roughly 60–80 nodes across the four stages, weighted toward the early
stages where lesson density is highest:

| Stage | Target | Character |
|---|---|---|
| pre-A1 | ~15 | survival exchanges: greeting, naming, courtesy, yes/no, farewell — mostly present |
| A1 | ~25 | the concrete everyday: wanting, having, going, location, time, number, family, food, simple description |
| A2 | ~25 | past and future reference, comparison, preference and opinion, sequences of events, transactions, plans |
| B1 | ~15 | narration at length, explanation and reasoning, hypotheticals, register shifts, sustained conversation repair |

These are planning targets, not quotas. A node is added because a real conversation
needs it, and the count is an outcome.

The concept taxonomy grows in step. Its 46 concepts are exactly exhausted by the
current 11 nodes; every new node brings new concepts, each with `family`, `gloss` and
`core`, and each becoming a cross-language join key.

## Per-track realization

Every one of the 20 `curriculum.json` files carries a `spine` ledger with an entry
for **every** spine node. Adding a node therefore touches all 20 tracks, and the
existing drift validators (`curriculum-segment-ledger-drift`,
`curriculum-omission-ledger-drift`, `curriculum-relocation-ledger-drift`) will fire
until each is updated.

This is intentional and should not be worked around. It is the mechanism that keeps a
track from silently falling behind the spine. But it means spine growth must be
**batched**, not dripped: adding one node at a time creates 20 ledger updates per
node. Nodes land in stage-sized tranches, with all 20 realizations updated in the same
change.

A track that genuinely cannot realise a node records it in `omits` with a reason, or
in `relocates` if it teaches the concept under a different node. Those ledgers are
recomputed caches — authors change lesson data, and the validator derives the ledger.

## Relationship to authored chapters

Spine expansion does not renumber, move, or rewrite chapters. Existing chapters keep
their lessons and their order. What changes is that a chapter beyond the current
spine's reach gains a shared node to point at through its `spineNodes` field in
`chapters.json`, instead of being describable only by language-specific extensions.

Where an authored chapter turns out to realise a capability the spine did not have,
that is evidence for a node — the corpus has been running ahead of its backbone, and
the node should be written to match what the tracks already teach rather than
inventing a parallel structure they must then be migrated onto.

## Validation

No new validation codes. Spine growth is already covered by the existing rules:
unknown node ids, cycles in the shared graph, ledger completeness and drift, and
`schema-v2-unknown-spine-node` on lessons. The gap report gains stage-coverage output
— nodes per stage, tracks realising each node, and concepts not yet owned by any node
— so expansion is measurable in the same way duration and prerequisite debt already are.

## Migration order

1. **A1 completion** — fill out A1 to its target and update all 20 ledgers.
2. **A2 tranche** — the largest single body of new nodes and concepts.
3. **B1 tranche.**
4. **Chapter attachment** — HL05 `spineNodes` fields updated across the 379 chapters
   as nodes become available.

Stages land in order because prerequisites run forward: an A2 node may depend on an A1
node, so A1 must be complete and realised first.

## Acceptance criteria

Spine expansion is complete when all four declared stages carry nodes; every node has
a first-person `canDo` and a non-empty concept set; every one of the 20 tracks carries
a ledger entry for every node, with omissions and relocations explicit and
non-drifting; every taxonomy concept is owned by exactly one node; the gap report
publishes stage coverage; and every one of the 379 chapters can name at least one
shared spine node it realises, or record why it is purely language-specific.

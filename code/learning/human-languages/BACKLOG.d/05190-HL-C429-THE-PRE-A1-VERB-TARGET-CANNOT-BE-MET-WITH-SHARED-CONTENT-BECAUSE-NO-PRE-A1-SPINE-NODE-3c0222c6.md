## HL-C429-3c0222c6 — The pre-A1 verb target cannot be met with shared content, because no pre-A1 spine node declares a verb concept

**Status: OPEN — a spine-staging question, deliberately not answered by a content
tranche.** Found while costing `verb-vocabulary` after HL-C427 made it visible
for the first time, by checking the mechanism before authoring against it.

### What was about to be done, and why it was wrong

HL-C427 revealed `verb-vocabulary` blocking **sixteen** tracks against a pre-A1
target of five. The obvious plan: a verb lesson is `type: word`, so it counts
toward `vocabulary` **and** `verb-vocabulary` — the only content that moves two
blockers at once. Roughly four verbs across fifteen tracks, sixty lessons, and
the cheapest criterion in the corpus falls.

Then the mechanism. `level-gate.ts:198`:

```ts
if (!CONTENT_TYPES.has(lesson.realization.type)) continue;
if (!/(^|-)VERB-/.test(lesson.realization.concept ?? "")) continue;
```

A verb is a lesson whose **`concept_tag` is spelled a certain way**, counted at
or below the level under test.

### The measurement that stops the plan

Every pre-A1 spine node, with its concept count:

| node | concepts | verb concepts |
|---|---|---|
| SPINE-MEET-GREET | 4 | **0** |
| SPINE-COURTESY-THANK | 3 | **0** |
| SPINE-RESPOND-BASIC | 4 | **0** |
| SPINE-EXCHANGE-NAMES | 10 | **0** |
| SPINE-CHECK-WELLBEING | 5 | **0** |
| SPINE-POLITE-REQUEST-REPAIR | 2 | **0** |
| SPINE-TAKE-LEAVE | 6 | **0** |

**Seven nodes, thirty-four concepts, not one verb.** Every shared verb concept
is declared by `SPINE-NAME-EVERYDAY-ACTIONS` (stage **A1**, fifteen verbs) or
`SPINE-SAY-WHAT-I-DO` (stage **A2**, twenty-four).

So a lesson tagged with a shared verb concept and filed on the node that owns it
derives to A1 or A2 and **cannot count toward pre-A1**. The criterion asks for
five verbs at a level where the shared spine declares none.

### How the eight passing tracks actually pass

Every pre-A1 lesson the gate counts as a verb, corpus-wide:

```
shared content       0
local support       18     shared name, filed off the owning node
concept unowned     63     a track-private name no node declares
```

**Zero of eighty-one are shared content.** Tamil's `TA-VERB-PO`, Telugu's
`TE-VERB-VELLU`, Spanish's `ES-VERB-ESTAR` are track-private names invented to
sit at pre-A1; Marathi's `VERB-GIVE` and Spanish's `VERB-SPEAK` use the shared
name but sit on `SPINE-POLITE-REQUEST-REPAIR`, which does not own it, so
`curriculum.ts:509` classifies them as local support.

Both routes are legal and both are in use. Neither is what the criterion looks
like it is asking for.

### Why this is not a content tranche

Authoring sixty lessons tagged `XX-VERB-<root>` would satisfy the gate on
fifteen tracks and would be **inventing per-track concept names to match a
regex**. It would also treble the 63 unowned tags into ~120 before anyone had
argued that the convention is right.

The question underneath is a curriculum judgement, not a mapping fix: **does
pre-A1 teach verbs?** HL09 §3.1 says yes, by asking for five. The shared spine
says no, by declaring none below A1. One of the two is wrong and they should be
reconciled before either is authored against.

### Three routes, none cheap

1. **Give a pre-A1 node verb concepts.** Most honest if §3.1 is right: the five
   verbs pre-A1 expects become shared concepts on a pre-A1 node, and the 63
   track-private tags become debt to migrate. Touches the shared 23-language
   spine, so it needs its own argument — see HL-C420 for what happens when
   concept ownership is moved without one.
2. **Restage nothing and accept the private-tag convention**, documenting it so
   authors stop inventing it independently. Cheapest, and it makes
   `verb-vocabulary` a measure of tag spelling rather than of the language.
3. **Change what the criterion counts.** Part of speech is a property of the
   word, not of the concept name; a `pos:` field or a verb ledger would measure
   the language instead of the metadata. Largest change, and the only one that
   makes the number mean what its name says.

### Method note, and it is the third time

HL-C418 recommended from a lesson's list entry rather than the lesson.
HL-C420 proposed moving lessons without reading what bound them to their node.
This entry was one command away from being the same mistake a third time: the
measurement (16 tracks, target 5, verbs count twice) was correct and the
recommendation built on it was wrong, because nothing had checked **how** the
criterion counts. `grep VERB- core/spine.d/*.json` was the difference.

**Do not author verb lessons for the pre-A1 gate until this is settled.**

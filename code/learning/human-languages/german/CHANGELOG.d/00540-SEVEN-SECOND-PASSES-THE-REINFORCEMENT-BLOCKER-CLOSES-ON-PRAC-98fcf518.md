## Seven second passes — the reinforcement blocker closes, on `practice` lessons rather than `review`

Seven lessons, none of which teaches anything.

| metric | before → after |
|---|---|
| pre-A1 atoms revisited fewer than twice | 37 → **0** |
| ladder blockers | 4 → **3** (`atom-budget 5`, `verb-vocabulary 2`, `vocabulary 201`) |
| lessons | 347 → 354 |
| atoms introduced | **0** |

`atom-budget 5` is **pre-existing** and was measured at the base commit rather
than assumed: it is not something these seven lessons caused.

### Thirty-seven atoms, fifty-six retrievals

The criterion is `revisits < 2`, and **19 of the 37 had zero**. An atom at zero
needs a retrieval in two *different* lessons, because the credit is counted per
lesson and a set counts a repeated id once:

```
19 x 2  +  18 x 1  =  56 retrieval slots
```

### `practice`, not `review`

German has **no `review` lesson anywhere** — 35 of its lessons are `practice`,
and that is the track's word for the same thing. Both types sit outside
`CONTENT_TYPES`, so neither adds a headword or moves the vocabulary blocker.
Copying another track's `type: review` would have introduced the first one into
a track that had never had one.

### Where the seven sit

```
GE-R07  ch11  spät, später, bis morgen — and the two marks inside them
GE-R14  ch27  what the calendar left lying there
GE-R17  ch30  wie geht es — and the whole ladder of answers
GE-R20  ch33  danke, bitte, Entschuldigung
GE-R29  ch42  nine sounds, said without looking at what they mean
GE-R30  ch43  Kopf und Haupt — two words for one head
GE-R31  ch44  the last few this book showed exactly once
```

Each lands on a path segment whose `spine_node` already matches its content —
the wellbeing ladder on `SPINE-CHECK-WELLBEING`, the courtesy set on
`SPINE-POLITE-REQUEST-REPAIR`, the sounds of the first meeting on
`SPINE-EXCHANGE-NAMES`. That is not decoration: a lesson's `spine_node` equals
its path segment's in **347 of 347** existing German lessons, so choosing the
segment chooses what the lesson is about.

Unlike Sanskrit, German's path rank and book sequence agree throughout — rank 20
at sequence 10, rank 610 at sequence 1050 — so there was no case here of a
lesson sitting early in the graph and late in the book.

### The one real finding: a phrase is not a claim

The `info-dump` gate rejected the first draft at **33 rule statements against a
ceiling of 32**, and the offending text was a warm-up line: *let the z be the ts
it always is in German, never an English z*. The detector reads `always` and
`never` as a rule being stated, and it is right to — but this was a pronunciation
cue wearing a rule's grammar, not a rule the lesson teaches.

Rewritten as an instruction — *let the z land as ts rather than as an English z*
— it says the same thing and costs nothing. That follows the precedent already
recorded in the gate's own comments for Russian, Persian and Sanskrit: **spend
the budget only on a lesson whose content genuinely is a rule.**

### What it cost the graph

Curriculum digest `aa7d8235`/7526 → `e4e3c864`/7533 on a 54-line structural
diff: the seven lessons joining their lists, one new extension node
(`GE-EXT-024-CONSOLIDATION`, for the only target segment that carried no
extension at all), and that id joining `GE-PATH-024`'s `after`.


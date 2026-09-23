## HL-C427-6d1d1e9d — The per-track ladder shows one blocker per track, so a track with two reads as having one

**Status: CLOSED (2026-09-23) — fixed, see RESOLVED below.** Found while clearing Telugu's reinforcement debt (HL-C426), by
reading the gate's JSON after the ladder had already been trusted for a
planning decision.

### What the ladder said, and what the gate held

```
telugu   touches A2  complete none  working pre-A1  vocabulary 221/300 — vocabulary short 79
```

One line, one criterion, and it reads as *one thing stands between this track
and pre-A1*. The gate's own record for the same run:

```json
"blockers": [
  {"criterion": "vocabulary",    "shortfall": 79},
  {"criterion": "reinforcement", "shortfall": 50}
]
```

`renderCurriculumGapReport` groups tracks **by their first blocker** and prints
each track once. Every criterion after the first is dropped.

### Why this is worse than a missing number

HL09 §3.1 is a **conjunction**: a level is attained only when all five criteria
pass. A report that shows the first failing criterion is showing a *necessary*
condition and presenting it as the *sufficient* one. Anyone planning from the
ladder — which is exactly what it was added for — will cost a track at its
vocabulary shortfall and be wrong by however many criteria are hiding behind
it.

Telugu is the measured case: the ladder said 79 words, the real answer was 79
words **and** 50 under-reinforced atoms, and the second of those turned out to
be the cheaper and the one that had to come first.

**How many other tracks are understated is unknown**, because the ladder cannot
show it. All 23 currently group under `vocabulary`, and that is the first
blocker for all 23, not the only one.

### The fix, and why it is not taken here

Print every blocker per track rather than the first. The shape is the open
question: the ladder's existing grouping is by criterion, and a track with two
blockers has to appear in two groups, or the grouping has to go. The honest
version is probably one row per track with each criterion's shortfall in its own
column, which loses the "nearest the next rung first" ordering the grouping
exists to give.

Deliberately not taken inside a content PR: it changes the report's shape and
the snapshots that pin it, and it should be argued on the layout rather than
landed as a side effect of authoring lessons.

### Do not fix by widening the sort

An earlier version of this ladder sorted every track on its bare shortfall and
had to be corrected, because shortfalls are in each criterion's own units — 79
headwords and 50 atoms are not comparable and neither is "nearer". Whatever
replaces the current layout must keep that: units do not compare across groups,
and a combined ordering across criteria would be the same error in a new place.

---

### MEASURED ACROSS THE CORPUS (2026-09-23), and it is worse than this entry estimated

This entry was filed from one track and guessed that "how many other tracks are
understated is unknown". HL-C428 took the measurement. Reading the gate per
track rather than the ladder:

| criterion | tracks it blocks |
|---|---|
| vocabulary | 23 |
| verb-vocabulary | 16 |
| reinforcement | 15 |
| atom-budget | 8 |
| spine-nodes | 2 |

**Sixty-four blockers across twenty-three tracks. The ladder shows
twenty-three**, one per track, and every one of them says `vocabulary`.

So the understatement is not occasional. **Every track is understated**, and two
whole criteria are invisible in the planning numbers this project works from:

- **verb-vocabulary** — the level wants five verbs at or below it, and sixteen
  tracks do not have them. It has never appeared in a plan.
- **atom-budget** — eight tracks: french, german, italian, kannada, malayalam,
  persian, tamil, urdu.

Two tracks — telugu and marwadi — are genuinely down to one blocker, and for
those two the ladder happens to be right.

**This raises the entry's severity.** Any per-track estimate taken from the
ladder, including "Telugu is 79 words from pre-A1", is a lower bound on a number
nobody has seen. The fix is unchanged; what has changed is that it should come
before the next planning decision rather than after it.

---

### RESOLVED (2026-09-23) — fixed, and the mechanism was worse than "shows one"

The defect was not that the ladder showed one blocker. It was **which** one:

```ts
const worst = [...track.blockers].sort((a, b) => b.shortfall - a.shortfall)[0];
```

It took the blocker with the numerically largest shortfall. Shortfalls are in
each criterion's own units and those units differ by two orders of magnitude:

| criterion | unit | observed range |
|---|---|---|
| vocabulary | headwords | 79 – 257 |
| reinforcement | atoms | 13 – 78 |
| verb-vocabulary | verbs | 1 – 29 |
| atom-budget | lessons | 1 – 5 |
| spine-nodes | nodes | 1 |

So `vocabulary` won on every track, every time, and **`verb-vocabulary` and
`atom-budget` were not occasionally hidden — they were unprintable.** No corpus
state could have produced a ladder row naming either of them. That is why
neither has ever appeared in a plan.

### The rule was already written down, one line from the code that broke it

`tests/level-gate.test.ts` said so itself:

> a first version that sorted on the bare number would have ranked a track
> needing one spine node above one needing 79 headwords

The guard against comparing across units was applied to the row **ordering** and
not to the **selection** of which blocker a row shows. Both live in the same
expression.

### The fix

Every blocker is printed. Grouping by criterion went with it — a track with four
blockers cannot sit in one group — and rows now order by **how many criteria are
left**, which is unit-free and is the honest reading of "nearest the next rung".

Within a row the blockers are sorted by criterion **name**, deliberately: any
ordering over the numbers would be the same mistake in a new place.

```
marwadi     … 1 blocker:  vocabulary 224
telugu      … 1 blocker:  vocabulary 79
hindi       … 2 blockers: reinforcement 44, vocabulary 101
…
german      … 4 blockers: atom-budget 5, reinforcement 37, verb-vocabulary 2, vocabulary 201
```

The old test asserted criterion groups were contiguous and shortfall ascended
within a group. Both were true of a one-blocker renderer, and both were
satisfiable while 41 of the corpus's 64 blockers went unmentioned. It is now
**set equality against the gate, per track**, in both directions, plus an
anti-vacuity assertion that more than one track renders more than one blocker.

Mutation-checked: restoring the `sort(...)[0]` selection fails on the first
track with two blockers, which today is twenty-one of twenty-three.

### What this changes downstream

Nothing in the corpus and every estimate taken from it. The planning numbers
this project has used are all lower bounds, and two criteria now enter planning
for the first time: **verb-vocabulary blocks sixteen tracks** and
**atom-budget eight**. Re-cost before the next content tranche rather than
after it.

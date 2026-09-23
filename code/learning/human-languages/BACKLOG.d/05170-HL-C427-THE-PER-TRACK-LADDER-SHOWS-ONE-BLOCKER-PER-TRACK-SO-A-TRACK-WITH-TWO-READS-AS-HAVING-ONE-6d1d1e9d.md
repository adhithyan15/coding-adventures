## HL-C427-6d1d1e9d — The per-track ladder shows one blocker per track, so a track with two reads as having one

**Status: OPEN.** Found while clearing Telugu's reinforcement debt (HL-C426), by
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

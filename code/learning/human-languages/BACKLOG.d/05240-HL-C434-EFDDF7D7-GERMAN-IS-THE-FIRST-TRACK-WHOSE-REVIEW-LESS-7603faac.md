## HL-C434-efddf7d7 — German is the first track whose review lessons had to be practice, and the info-dump gate catches a phrase not a claim

**Status: CLOSED (2026-09-23) — implemented.** The second of the eight large
reinforcement debts, applying HL-C433's slot arithmetic for the first time from
the start rather than discovering it mid-tranche.

```
german: reinforcement 37 -> 0   (seven lessons)
```

**Seventeen of twenty-three tracks** now carry no pre-A1 reinforcement debt.
German goes 4 ladder blockers to 3; `atom-budget 5` survives and is
**pre-existing**, measured at the base commit rather than assumed.

### THE SLOT RULE HELD, AND SIZED THE TRANCHE CORRECTLY UP FRONT

```
19 zero-revisit x 2  +  18 one-revisit x 1  =  56 retrieval slots  ->  seven lessons
```

HL-C433 discovered this rule halfway through Sanskrit. Here it was applied
before a line was written, and the estimate — seven lessons — was what shipped.
That is the first evidence the rule generalises rather than describing Sanskrit.

### A TRACK'S REVIEW TYPE IS A TRACK PROPERTY

**German has no `review` lesson anywhere.** Its 347 lessons include 35
`practice` and zero `review`. Both types are outside `CONTENT_TYPES`, so either
satisfies the reinforcement criterion without adding a headword — but copying
Sanskrit's `type: review` would have introduced the first `review` lesson into a
track that has never had one, in a tranche whose entire point is that it changes
nothing about the track except retrieval.

Check `grep -h "^type:" lessons/*.md | sort | uniq -c` before authoring. The
answer differs per track and nothing enforces consistency across tracks.

### THE INFO-DUMP GATE REJECTED A PHRASE, NOT A CLAIM

The first draft came in at **33 rule statements against a ceiling of 32**. The
finding:

```
GE-R31 rule-statement always-never:
  "let the **z** be the *ts* it always is in German, never an English *z*"
```

The detector keys on `always` / `never`, and it is right to. But this is a
**pronunciation cue wearing a rule's grammar** — the lesson does not teach a
rule about z, it tells you where to put your tongue. Rewritten as an
instruction (*let the z land as ts rather than as an English z*) it says exactly
the same thing and costs nothing.

That is the precedent the gate's own comments already record for Russian,
Persian and Sanskrit: **spend the budget only on a lesson whose content
genuinely IS a rule**, and rewrite everything else. The ceiling is documented as
debt that may fall and never grow, so absorbing an incidental phrase into it
would have been the wrong trade.

### PLACEMENT WAS EASIER THAN SANSKRIT'S, FOR A REASON WORTH RECORDING

German's path rank and book sequence **agree throughout** — rank 20 at sequence
10, rank 610 at sequence 1050. Sanskrit's script lessons sat at rank 40 while
sitting near the end of the book, so the two orderings pointed opposite ways and
every placement had to be checked against both. Here one check sufficed.

The `spine_node` rule held again independently: **347 of 347** German lessons
have a `spine_node` equal to their path segment's. Two tracks, two confirmations,
zero exceptions — this is a hard constraint, not a convention.

One placement detail cost a new node: `GE-PATH-024` carried **no extension at
all**, and a `practice` lesson is local support that must hang from one, so
`GE-EXT-024-CONSOLIDATION` was added. Every other segment reused what it had.

### Remaining

```
punjabi 40   kannada 41   hindi 44
malayalam 44   tamil 73   arabic 78
```

320 atoms across six tracks. Sized in slots rather than atoms, **kannada is the
cheapest left** (41 atoms, only 3 zero-revisit, 44 slots) despite having the
most atoms of the small three — but it spreads over 24 chapters, which is the
other half of the cost. **Arabic still wants its own entry**: 146 slots, 68 of
78 with no later revisit at all, which is a missing review layer rather than
debt.

## HL-C246 — Tamil needs a distant-review band; chapter recaps cannot reach R3/R4

Tamil's script-closure debt is **zero** and its hand-written chapters are gone, so
the next constraint on this track is retrieval distance, and it is now the only
number moving the wrong way.

### What the flip cost, measured

`migrate_schema_v2.py` assigns **one atom per lesson**, and it is honest about
that being an under-count. Migrating chapters 1-5 therefore introduced 33 atoms
that nothing else in the corpus practised:

```
                        before flip   after migration   after recap wiring
atomsTaught                     367               399                  400
atomsNeverRevisited              50                82                   54
retrieval misses                981              1109                 1084
  R1                            131               163                  151
  R2                            283               315                  302
  R3                            313               345                  346
  R4                            254               286                  285
```

Wiring the five chapter recaps to practise their own chapter's atoms — declared
block by block, so the gate checks the recap really retrieves them rather than
just claiming them in frontmatter — recovered most of R1 and R2 and two thirds of
the never-revisited regression. **It cannot touch R3 or R4 and never could.**

### Why, structurally

`REINFORCEMENT_WINDOWS` puts R3 at 20-60 lessons out and R4 at 80-250. A chapter
recap sits **three to ten** lessons from the material it reviews. Every recap in
the track, present and future, lands inside R1/R2 and outside R3/R4 by
construction. Adding more recaps cannot close these windows; it can only make R1
look better.

R3 = 346 and R4 = 285 are therefore not a backlog of missing recaps. They are the
absence of a review structure at the distance the windows measure.

### The shape that is known to work

Gujarati's band, per the tranche note: **one chapter of nine zero-new-atom
lessons returning material 98-104 positions out**, plus a named distant band per
later chapter. It moved that track's retrieval misses 339 → 283, R4 101 → 43 and
atoms-never-revisited 5 → 1, **while** growing vocabulary 52 → 72 and raising
ear-drivable 56% → 65%. Growth and review were not in tension.

Tamil is 317 lessons over 66 chapters, so it has the runway for the same shape
several times over. A band placed around chapter 30 reaches chapters 1-6 at R4
distance; one around chapter 50 reaches the twenties.

### Two constraints whoever picks this up should know first

1. **A zero-new-atom lesson is voice-cored if it drills, and the drills already
   exist.** These bands are pure retrieval, so unlike script lessons they cost
   nothing in drivability — Tamil's reach is 208 and should come out higher, not
   lower.
2. **`practises.knowledge` in frontmatter is not enough.** `validate` requires
   every practised atom to be assessed by a named body block, and requires it to
   be transitively available through `prerequisites`. That is the right rule and
   it is why the recap wiring above took three passes. Budget for it.

### Also left behind, smaller

* **`cognates` has no generated equivalent.** Four Tamil tables now render inside
  `cousinweb`, which is the right box by title but loses the violet border the
  hand-written chapters used to distinguish a family table from an etymology. If
  the distinction is worth keeping, it is a generator change (a `## Across the
  family` heading mapping to its own environment), not a Tamil change — and it
  would serve every Indic track at once.
* **`TA-C03-eppadi-irukkirirgal` forward-references இரு by 118 lessons.** It is
  the oldest forward reference in the track and survived this tranche untouched.
  `TA-C32-iru` teaches the verb; the chapter-3 lesson spends it.

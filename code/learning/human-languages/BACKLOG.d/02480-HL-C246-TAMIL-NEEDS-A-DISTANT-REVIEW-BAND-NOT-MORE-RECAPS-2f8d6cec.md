## HL-C246 — three findings from the Tamil retirement tranche

One file, three findings, because they came out of one pass and a reader chasing
any of them will want the other two. In order: the retrieval-distance gap, the
one-lesson-one-chapter measurement, and the glyph ladder's real shape.

### A. Tamil needs a distant-review band; chapter recaps cannot reach R3/R4

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


---

### B. One lesson = one chapter needs NO policy change. Measured, not assumed.

The owner asked for every content lesson to be its own chapter, with practice and
re-emphasis chapters placed routinely through the run, and asked whether HL05's
payoff rule and `payoffRepresentativeness: 0.5` survive a chapter that has no
separate payoff lesson.

**They survive untouched, and the corpus already proves it.**

```
single-lesson chapters already in the corpus        295
  of those, PASSING the payoff rule                 277  (94%)
  of those, failing it                               18  (all spanish)
```

A single-lesson chapter is its own payoff: the chapter's introduced atoms are
that lesson's introduced atoms, and the lesson's own `Guided Practice` /
`Wrap-up Recall` blocks assess them. Representativeness comes out at 1.00 for any
well-formed schema-v2 lesson.

The 18 failures are not structural. Every one reads *"payoff assesses 1/3 of the
chapter's atoms (0.33)"* — a lesson that introduces three atoms and assesses one
in its own body. That is a lesson defect the rule correctly catches, and it
exists independently of chapter shape. **No spec or policy change is required;
`chapter-policy.json` already says length is never a cost.**

Worth knowing before anyone argues the flat shape is novel: **Spanish already
runs 417 chapters over 1059 lessons** — 2.5 lessons per chapter. The flat shape
is the majority pattern in the corpus's most advanced track, not a departure.

Tamil's current shape, for comparison:

```
66 chapters, 317 lessons, mean 4.8
sizes: 1x3  2x9  3x6  4x7  5x28  6x5  7x2  8x2  9x1  10x1  11x1  20x1
```

**The ripple, measured, so the next agent can scope it.** Flattening chapters 1-5
alone (36 lessons) makes 36 chapters and renumbers 6-66 to 37-97:

| what moves | count |
|---|---|
| lesson `chapter:` fields | 317 (36 rewritten, 281 shifted) |
| `tamil/chapters.d/*.json` | 67 → 98 |
| `core/book-generation.d/targets.d/tamil-*.json` | 66 → 97, each with a renamed `output` |
| `tamil/book/chapters/*.tex` | 67 renamed |
| generated book/modality/narration/snapshot owners | all, keyed by chapter |
| **chapter-number references in lesson prose** | **53, across 31 lessons** |

That last row is the only genuinely hard part and it is why this was not attempted
in the retirement tranche: they are human sentences — *"In Chapter 5 you said…"* —
that all become wrong, `chapter-references.test.ts` ratchets their count so they
cannot simply be added to, and a mis-remapped one silently misleads a reader
rather than failing a gate.

Retiring chapters 1-5 into the pipeline is the precondition and is now done: you
cannot flatten a chapter the generator skips.

---

### C. The glyph ladder is worse than "batched". It is anti-correlated with its own rule.

The owner asked why the ladder does not jump to **ன** after *magiḻcci*. Traced,
and the answer is that Tamil's script strand is not paced at all. Measured over
the 68 script lessons in reading order:

```
gaps between consecutive script lessons, histogram (10 = 10+)
  1 lesson  : 30      <-- back to back
  2 lessons : 13
  3 lessons : 5
  4 lessons : 12
  5 lessons : 1
  6+        : 7
```

**43 of the 67 gaps (64%) are closer than `minLessonsBetweenScriptSegments: 2`
allows, and 30 of those are back-to-back.** So the ladder is not "absent then
batched" — it is overwhelmingly clustered, with the remainder swallowed by two
enormous holes:

| gap | between |
|---|---|
| **129 lessons** | `TA-W20-read-onru` → `TA-W21-read-kudi` |
| **27 lessons** | `TA-W00-va-guided-copy` → `TA-W01-curves-va-ka` |

The 27-lesson hole at the front is the one the owner spotted, and *magiḻcci* sits
inside it. **The 129-lesson hole in the middle is larger and nobody had seen it.**

Note what the retirement tranche did and did not do here. Chapters 1-6 no longer
*show* untaught script, so the front hole is no longer a correctness problem —
closure is 0. It is still a **pacing** problem, and pacing is what the owner is
objecting to. Closure being zero is exactly what makes reseating safe to attempt:
there is no debt to protect while the ladder moves.

Two constraints for whoever reseats it:

1. **Closure is measured in reading order.** Every glyph must still land before
   its first consumer. `TA-S125-letter-u` at sequence 492 is the worked example:
   as late as its own glyphs allow, still ahead of chapter 9.
2. **A script lesson is pen by definition** (`modality.ts`), so where each one
   lands is the only lever on drivable reach. Tamil's reach is 208; spreading 68
   pen lessons more evenly will move it, and the direction depends entirely on
   whether each lands before or after its chapter's voice-cored opening.

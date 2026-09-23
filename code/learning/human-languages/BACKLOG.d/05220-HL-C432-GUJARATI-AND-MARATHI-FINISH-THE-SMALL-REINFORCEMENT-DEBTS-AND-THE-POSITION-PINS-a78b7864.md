## HL-C432-a78b7864 — Gujarati and Marathi finish the small reinforcement debts, and the position pins turned out compatible with the fix

**Status: CLOSED (2026-09-23) — implemented.** The two tracks deferred twice for
their reinforcement-window position pins, and the last of the small debts.

```
gujarati: reinforcement 13 -> 0   (one lesson)
marathi:  reinforcement 14 -> 0   (two lessons)
```

**Fifteen of twenty-three tracks now carry no pre-A1 reinforcement debt**, up
from three when HL-C428 started. Every track with sixteen or fewer thin atoms is
done; what remains is the eight large ones, 385 atoms.

### ONE OF THE TWO DEFERRALS WAS BASED ON A MISREAD TITLE

Marathi was deferred twice on the strength of a test **name**:

> `gives the five Chapter 14 family atoms genuine R1, R2, and R3 retrieval`

The body is a set-emptiness assertion on five named atoms:

```ts
expect(report.reinforcement.filter((d) => repairedAtoms.has(d.atom))).toEqual([]);
```

Adding a review lesson can only give those atoms **more** revisits, so it cannot
break this. Marathi has no index pin and no whole-track total; it was as easy as
bengali. **Two tranches of deferral bought nothing.** Read the body, not the
title.

### Gujarati's pins are real, and they turned out not to conflict

Five tests assert exact indexes — `ordered[134]` by id, the nine doorway
distances `[108, 107, … 100]`, and four bridges at positions 111-133:

```ts
const checkpoint = ordered[134]!;
expect(checkpoint.realization.lessonId).toBe("GU-R19-doorway-nine-r4");
```

**Measured: gujarati index 134 is sequence 870.** Anything inserted below that
shifts all five.

The constraint and the content happened to be compatible. All thirteen thin
atoms sit at or below sequence **1820**, and the track runs to 2290 — so one
lesson at **1825** is after every atom it retrieves *and* at index 220, far past
the checkpoint. **One lesson, thirteen atoms, no position assertion touched.**

That is luck rather than design, and it is worth writing down: the next track
with index pins may not be so lucky, and the check is cheap — find the sequence
at the pinned index, compare it to the highest thin-atom sequence.

### The missed-window total, decomposed

The expensive pin was not an index but a whole-track count:

```ts
expect(afterCheckpoint.reinforcement.flatMap((d) => d.missed)).toHaveLength(359);
```

Its comment decomposes every historical change into named categories, and that
convention is the work. Measured by diffing the `(atom, window)` pairs before
and against after, **359 → 350**:

```
-10  closed outright by the lesson's thirteen retrievals
       nine R4: the greeting read, saarun, the chapter-2 and chapter-6 recaps,
                the three standing vowels, kha, ddha
       one  R2: GU-SCRIPT-KAAGAL-01, still open at distance ~6
 +1  PRE-EXISTING, created by the track growing 280 -> 281:
       GU-LEX-KERI-01's R4 (distance 80-250) did not exist at 280 lessons and
       does at 281. Nothing about that lesson changed.
```

Every one of the eleven moved slots is accounted for. Re-pinning 359 to 350
without that would have been the re-baselining HL-C425 warned about.

### A sixth rule a review lesson trips

Gujarati pins a **session map** — `session-map.md`, a table of contiguous
session ranges — and the test asserts the ids in it equal the track's lessons in
sequence order, with every range abutting the next. Adding one lesson to chapter
35 required inserting its id *and renumbering ten later rows*. It is the first
pin in this programme that lives in prose rather than in a test file.

### Remaining

```
sanskrit 28   german 37   punjabi 40   kannada 41
hindi    44   malayalam 44   tamil 73   arabic 78
```

385 atoms across eight tracks. **Arabic still wants its own entry**: 68 of its 78
have no later revisit at all, in a track of 129 lessons, which is a missing
review layer rather than debt and should not be answered with this template.

## HL-C286 — Kannada chapter 3 is gentle per LESSON and over budget per CHAPTER

Retiring `kannada` chapter 3 moved it from schema v1 (which declares no atoms at
all, so it was invisible to every ramp measurement) to v2, and the chapter
immediately appeared as the track's **first** `atomChapterSpikes` entry:
`0 -> 1`. `atomsNeverRevisited` moved `9 -> 11` in the same pass.

**Nothing got steeper. Something previously unmeasured became measurable.** This
is the expected shape of a v1->v2 migration and should be read as coverage
arriving, not as a regression — but it is worth writing down, because the number
that moved is the one a future reader will use to judge the chapter.

### The two budgets disagree, and the one that defines "gentle" passes

`core/chapter-policy.json`:

    maxNewAtomsPerLesson:  3     <- chapter 3's worst lesson introduces exactly 3
    maxNewAtomsPerChapter: 12    <- chapter 3 introduces 15

Fifteen atoms across eight lessons is **1.9 per lesson**, under the corpus mean
of 2.31. The chapter introduces **five new words across six content lessons** —
one new word per lesson, which is the owner's actual rule. Each word carries a
meaning atom, usually an etymology atom, and sometimes a grammar atom, which is
the convention the already-generated `KA-C06-*` lessons established
(`KA-LEX-…`, `KA-ETYMON-…`, `KA-GRAMMAR-…`). Two more atoms come from the two
script lessons that sit in the chapter.

So the chapter total is over budget because the chapter is LONG, not because any
step in it is large.

### Why the atoms were not merged to get under the number

The arithmetic fix is easy and wrong: fold each `KA-LEX-…`/`KA-ETYMON-…` pair
into one atom and the chapter lands on 12 exactly. That would make the number
compliant by making the accounting coarser, while the reader's burden stayed
identical — the same failure as a metric that never moves, pointed the other way.
"nānu means I" and "nānu is Proto-Dravidian, and is NOT cousin to English me"
are two things a reader can know independently, and the exam can ask for either.

### The real remedy, and why it is not in the retirement PR

`maxNewAtomsPerChapter` is a *chapter-length* budget, and the standing directive
is that length is never a cost — `chapter-policy.json` says so in its own note:
"no threshold here may penalise page, lesson, or chapter count." The honest
remedy is therefore to **split chapter 3**, not to teach less in it: the
how-are-you exchange (hēge, hēgiddīrā, cennāgi) and the courtesy reply
(paravāgilla) are already two spine nodes, `SPINE-CHECK-WELLBEING` and
`SPINE-COURTESY-THANK`, sitting in one chapter.

That is a structural change that renumbers every later chapter and ripples into
`language-ladder`, which hardcodes chapter and lesson counts. Doing it inside a
chapter-retirement PR would make the PR large and slow enough to rot against a
main branch that moves several times an hour, so it is recorded here instead.

**Open question this raises for the other 33 hand-written chapters:** every one
of them is schema v1 today, so every one of them is invisible to
`atomChapterSpikes` in exactly this way. The corpus-wide spike count is not a
measurement of the corpus; it is a measurement of the migrated part of it, and
it will keep rising as retirement proceeds. That rise is progress, and whoever
reads the trend next should know that before treating it as decay.

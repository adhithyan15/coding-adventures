## HL-C314 — a sub-population is not a band, and the opening chapters are debt

German chapter 6 first shipped as one chapter of **32 atoms** against a
`maxNewAtomsPerChapter` of **12**. The justification was that it sat in family
with its neighbours: German's opening chapters run **31, 23, 36, 30, 30**.

Every number in that sentence is true and the conclusion was wrong, because
those five chapters are not German's band — they are the five chapters next to
the one being sized.

### The measurement that settles it

Across all 29 atom-bearing German chapters at the time:

| | German | French |
|---|---|---|
| median | **9** | 9 |
| mean | 12.7 | 8.5 |
| max | 36 | 12 |
| over the ceiling | 6 of 29 | 0 of 37 |

**German's median is the same as French's.** The six over-budget chapters are
1–6; the other 23 already sat at or under the ceiling. Chapter 6 was split into
four (9, 9, 8, 6) and `atomChapterSpikes` moved **6 → 5**.

The generalisable form: **when a number looks acceptable "in family", state
which family you measured.** Adjacent chapters are the population most likely to
share whatever produced the outlier, so comparing against them is the one
comparison guaranteed to ratify it. Measure the track. The band is per-track and
it has to be recomputed, not remembered — French's distribution justified three
splits on exactly the same rule that German's initially seemed to excuse.

### THE OPEN DECISION: German chapters 1–5

These are **already generated and merged**, and nobody has decided whether they
get split:

| chapter | atoms | lessons |
|---|---|---|
| 3 How Are You | **36** | 18 |
| 1 Greetings | **31** | 14 |
| 4 Farewells | **30** | 16 |
| 5 The First Verbs | **30** | 15 |
| 2 Introducing Yourself | **23** | 10 |

Each is two to three times the ceiling. They are the last German chapters above
it, and they are the reason German's mean (11.5) sits so far above its median
(9).

Three things make this a real editorial decision rather than an obvious fix:

* **They are shipped.** Splitting renumbers the whole track, and every split so
  far has renumbered everything above it.
* **The ceiling is report-only by design.** `chapter-policy.json` says these
  budgets ship report-only "because the corpus predates them and a gate that
  fails on recorded debt teaches authors to route around it". Chapters 1–5 are
  exactly that recorded debt.
* **The opening of a course is the place a dense chapter hurts most.** A reader
  five lessons in has the least ability to absorb 36 atoms, which argues for
  splitting rather than grandfathering.

The same question exists for every track's opening chapters; German is simply
the first place it has been measured. Not in scope for the hand-written
retirement, which is why it is recorded here rather than fixed in passing.

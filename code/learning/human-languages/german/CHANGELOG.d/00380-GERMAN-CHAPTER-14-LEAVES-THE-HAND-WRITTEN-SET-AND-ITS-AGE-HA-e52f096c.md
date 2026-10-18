## German chapter 14 leaves the hand-written set, and its age half moves to where the copula is

`ch14-haben-alter.tex` is now generated — as **chapter 14, *To Have***, plus a
new **chapter 19, *Being Your Years***, carrying the half of it that could not
stay. German's hand-written chapters: **9 -> 8**. Old chapters 19-35 renumber to
20-36.

### Sizing it four ways

| instrument | answer |
|---|---|
| `handwritten_parity.py` | gap of **6** |
| `grep -l '^chapter: 14$' lessons/*.md` | **2 lessons** |
| the `.tex` | **2 sections** |
| taught-form census | *haben* x6 forms, *habēre*, *alt*, *Jahr/Jahre*, *einen* |

Whole, the chapter is **~17 atoms against a ceiling of 12**. Split at the seam
the ramp already dictated, it is **8 and 6** — and chapter 14 then needs no
split of its own.

### The age sentence moved rather than carrying a documented forward reference

*Ich bin zwanzig Jahre alt* needs *ich bin*; *Wie alt bist du?* needs *du bist*.
Both are chapter 18. Teaching the sentence four chapters before the copula is
the ramp violation this programme exists to remove, and accepting a documented
forward reference would have bought nothing, so the material moved instead.

It could not join chapter 18 — that chapter introduces 9 atoms and the age block
is 6, which is 15 against a ceiling of 12 — so it is a chapter of its own,
placed **immediately after** it. That interrupts the *sein* arc on purpose: a
reader who has just met six forms of a verb should use them for something real
before meeting its past. Parking it at the end of the book would have renumbered
nothing and divorced the sentence from both verbs it argues about; that was the
cheap option and it is not the one taken.

### Four regular cells share a lesson, and the two irregular ones do not

Chapter 5 is now schema-v2, so the weak ending machine is an **atom this chapter
requires** rather than a pattern it assumes. On *hab-* that machine is right four
times out of six. Those four share one lesson which makes the reader *build*
them — ending first, then the form, then the same machine run on *wohn-* to show
it is not about this verb. *Du hast* and *er hat* get a lesson each, because
nothing generates them.

This is a different call from chapter 15's *Perfekt*, and deliberately so: there
no stated rule existed and the fourth cell would have been a guess, which is what
`maxNewGrammarCellsPerLesson` forbids. Here the learner applies a rule they own.

### The running example nobody had taught

*Ich habe einen Bruder* is the `.tex`'s own first example, and ***einen* is a
headword nowhere in German**. It is taught here as one atom — the form for a
masculine thing you have — flagged as the first sighting of a system that gets
its own chapter later, not as a case lesson.

Teaching it surfaced a **real defect in chapter 1**: `GE-C01-guten-tag` cited
*ich wünsche einen guten Tag* to explain the frozen *-en*, which was an 89-lesson
preview that no gate could see, because the gate cannot flag a reference to a
word nothing teaches. `continuity.test.ts` pins German chapter 1 at **zero**
previews, and this change made the existing one visible. Chapter 1 now makes the
same point in English and keeps every teaching claim.

### Counters, re-measured against the merged tree

| Measure | Before | After |
|---|---|---|
| German hand-written chapters | 9 | **8** |
| corpus hand-written chapters | 18 | **17** |
| `handwritten_parity.py` german | 46 | **40** blocks at risk |
| German lessons (schema-v2) | 166 | **181** |
| atoms taught | 321 | **335** |
| atom-measurement-blind lessons | 18 | **16** |
| chapters over the 12-atom budget | 5 | **5** |
| culture claims | 16 | **18** |
| atoms never revisited | 81 | **78** |
| forward references | 36 | **35** |
| cross-chapter prose references | 0 | **0** |
| book pages | 327 | **346** |

`paradigmTables` (95), `lessonsWithFindings` (121), `ruleStatements` (30) and
`fullParadigmGrids` (**21**) are all unchanged: `GE-C14-haben`'s grid finding
left and `GE-C14-practice`'s recap replaced it, and the recap uses the
`singular | plural` shape so it stays under `FULL_GRID_ROWS` — German does not
get a full paradigm back and chapter 5's inverted `GE-C05-wohnen` fixture stays
valid.

Forward references moved **both ways** and netted -1: two retired (*ich bin* and
*sein*, now behind the age chapter rather than ahead of it), two newly exposed by
teaching words nothing had taught (*Jahr* in `GE-C03-gehen`, *einen* in
`GE-C01-guten-tag`), and one of those two then fixed in chapter 1. The *Jahr*
one was kept and paid off: `GE-C03-gehen` names *Jahr* as an example of the
silent lengthening *h*, and `GE-C14-jahr` now cashes that in instead of leaving
it a promise.

### Verification

124/124 test files, 1746 tests; all eleven `check:*` gates; language-ladder
39/39 files, 442 tests; the German book compiles under XeLaTeX with zero errors,
zero overfull or underfull boxes and zero missing characters (346 pages), and
chapters 14 and 19 were read on the page. All 29 teaching claims in
`ch14-haben-alter.tex` were checked across into the new lessons.


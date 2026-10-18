## German chapter 3 leaves the hand-written set

Chapter 3 — the *Wie geht es dir?* chapter — was hand-written LaTeX. It is now
generated from its lessons, and generating it turned out to require real
authoring rather than a format flip.

### The chapter had a hole, and the flip exposed it

Eight lessons carried a chapter whose LaTeX taught roughly twenty things. The
gap was not padding: *es*, *mir*, *dir*, *Ihnen*, *sehr*, *nicht*, *und*, *so
lala*, *schön* and *vielen Dank* were all taught in the book and owned by no
lesson. `GE-C03-gehen` alone introduced *gehen*, *es*, *mir* and the dative in
one sitting — four new things in a lesson budgeted for three.

So the chapter was authored out to **18 lessons**, one new word each:

- **Ten new lessons** — `schoen`, `vielen-dank`, `es`, `mir`, `dir`, `ihnen`,
  `sehr`, `nicht`, `so-lala`, `und`.
- **Eight migrated** from schema v1 to v2, with the material that now has its
  own lesson removed from them. `GE-C03-gehen` keeps only *gehen*; the "dressing
  it up" list left `danke` for the three lessons that own its parts.

Every lesson is within `maxNewAtomsPerLesson: 3`, and the chapter introduces
**36 atoms**.

### What moved

- `core/book-generation.d/handwritten.d/german-0003.json` is now a `targets.d/`
  entry. German's hand-written chapters: **14 -> 13**; corpus-wide **38 -> 37**.
- `handwritten_parity.py german`: **77 -> 69** blocks at risk. Chapter 3's own
  gap went 8 -> 0 by being generated; before the flip it had already fallen 8 ->
  3 as the prose was carried into lessons. The residue was two `morphologybox`
  environments and one `grammarlens`, none of which the generator can emit —
  their content was re-homed into `cousinweb` tables (the *ich/mich/mir* and
  *doer/to-whom* grids) and a `grammarlens` on returning the question.
- The payoff is atom-scored instead of authored boilerplate: 23 of the 36 atoms
  — the lexical spine, the dative, and the two pragmatic rules. Per-word sound
  and etymology atoms are deliberately not claimed.
- Chapter 3's lessons are now in the curriculum path (`GE-PATH-010`,
  `GE-PATH-012`) and three new extension nodes carry the language-specific and
  consolidation lessons.

### Counters, re-measured rather than derived

All of these were regenerated against the merged tree, not adjusted by
arithmetic:

| Measure | Before | After |
|---|---|---|
| atoms taught | 177 | 213 |
| lessons with measured budgets | 64 | 82 |
| atom-measurement-blind lessons | 46 | 38 |
| chapters over the 12-atom budget | 2 | 3 |
| culture claims | 6 | 8 |

Chapter 3 joining the over-budget list is a **measurement, not a regression** —
the same debt chapters 1 and 2 surfaced. The honest fix stays a chapter split,
which renumbers every later German chapter, and is still deliberately left open
as HL-C242.

### Two defects caught by reading the rendered page

A block count cannot see either of these, and both were found by compiling the
book and looking at it.

1. **The dialogue ran on.** Both practice lessons wrote their four-line exchange
   as consecutive `>` lines. Markdown joins those into one paragraph, so
   *Hallo! Wie geht's? — Gut, danke, und dir? — Es geht.* printed as a single
   run-on line instead of an exchange. Chapter 2 already had the answer — a
   two-column German/English table, one line per row — and both chapter 3
   dialogues now use it.
2. **A wrong sound tag.** `GE-C03-gehen` declared `h-pronounced` while its own
   prose said the *h* "merely lengthens the vowel." The registry has the right
   tag, `h-silent-lengthening`, and the lesson now uses it and teaches the rule
   as transferable — *sehr*, *Ihnen* and *Jahr* all hide the same silent *h*.

### One deliberate wart

Chapter 3 introduces *gehen* first, but `GE-C27-gehen` already owned
`GE-LEX-GEHEN-02` and three chapter-27 lessons require it. Rather than
re-point a generated chapter's atoms, chapter 3 mints `GE-LEX-GEHEN-01` for the
wellbeing verb and chapter 27 keeps `GE-LEX-GEHEN-02` for the motion verb. Two
atoms for one headword is not ideal; folding them belongs with the chapter-27
work, not here.



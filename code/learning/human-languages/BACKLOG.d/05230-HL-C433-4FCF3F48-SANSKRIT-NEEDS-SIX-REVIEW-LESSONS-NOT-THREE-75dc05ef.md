## HL-C433-4fcf3f48 — Sanskrit needs six review lessons, not three, because a zero-revisit atom costs two

**Status: CLOSED (2026-09-23) — implemented.** The first of the eight large
reinforcement debts, and the first tranche in this programme where the arithmetic
that sized every previous one stopped working.

```
sanskrit: reinforcement 28 -> 0   (six lessons)
```

**Sixteen of twenty-three tracks now carry no pre-A1 reinforcement debt.**
Sanskrit drops from three ladder blockers to two: `verb-vocabulary 5`,
`vocabulary 116`.

### THE SIZING RULE THE SMALL TRANCHES HID

Every tranche from HL-C426 to HL-C432 sized itself as *one retrieval per thin
atom*, and every one of them was right, because every one of them met only atoms
whose `revisits` was already **1**. Telugu's fifty were fifty ones. Gujarati's
thirteen fit in a single lesson for the same reason.

The criterion is `revisits < 2`. An atom at **zero** needs **two** further
lessons, and they must be two DISTINCT lessons — a second mention inside one
lesson buys nothing, because `practisedAtoms` is a set per lesson.

Sanskrit's 28 split 17 zero / 11 one, so the true cost is not 28 but

```
17 x 2  +  11 x 1  =  45 retrieval slots
```

and 45 slots at the observed ceiling of ~10 atoms per review lesson is **six
lessons**, not three. Measured across the remaining tracks the zero-revisit share
is what predicts effort, not the headline atom count:

| track | thin | zero-revisit | slots | chapters spanned |
|---|---|---|---|---|
| sanskrit | 28 | 17 | 45 | 8 |
| german | 37 | 19 | 56 | 12 |
| punjabi | 40 | 11 | 51 | 18 |
| kannada | 41 | **3** | 44 | 24 |
| arabic | 78 | **68** | 146 | 15 |

Kannada has the most atoms of the four small ones and nearly the fewest slots.
Arabic's 146 is why it is not a tranche of this template.

### CHAPTER SPREAD IS THE OTHER HALF OF THE COST

A review lesson must sit after every atom it retrieves and live in one coherent
chapter, so a debt spread over 24 chapters cannot be answered as cheaply as the
same debt spread over 8, whatever the slot count says. Sanskrit was picked first
on that basis rather than on being the smallest.

### THE SIX, AND WHY THEY SIT WHERE THEY DO

```
SA-R06  ch 6  seq  335  SA-PATH-009          where the words came from
SA-R11  ch11  seq  545  SA-PATH-014          the whole first meeting
SA-R12  ch12  seq  575  SA-PATH-015          what Sanskrit leaves out
SA-R13  ch13  seq  595  SA-PATH-016          five parts, three kinds of ancestry
SA-R29  ch29  seq 1425  SA-PATH-29-COURTESY  how this book argues
SA-R30  ch30  seq 1475  SA-PATH-30-WELCOME   from the door to the promise
```

Two placement constraints, and they are independent:

- **Book order** — a retrieval must come after the `sequence` that introduced
  the atom. The two script atoms are introduced at sequences 1281 and 1341, so
  their retrievals can only be the ch29 and ch30 lessons.
- **Curriculum-graph order** — `validate` judges prerequisites on path-file rank
  plus `pathOrder`, which is NOT sequence. The script lessons illustrate the trap
  in reverse: `SA-S223` and `SA-S224` sit on `SA-PATH-100` at **rank 40**, near
  the front of the graph, while sitting near the end of the book.

`spine_node` is not free either. Measured over all 353 existing lessons, a
lesson's `spine_node` equals its path segment's in **353 of 353** cases, so
choosing the segment chooses the node, and the node has to suit the content.
That is what drove each lesson to the segment it is on rather than to the
chapter that would otherwise have been convenient.

### THE CHEAPEST STRUCTURAL DIFF THIS PIN HAS SEEN

The curriculum-graph digest moves `87df31a5`/7520 -> `aa7d8235`/7526, and the
structural diff is **36 lines for six lessons** where HL-C431 needed 63 for the
same count. Every one of the six reuses the extension its path segment already
carried, so there is not one new extension node in the diff — only the six ids
joining derived `lessons` lists, twice each, and six trailing commas.

### Remaining

```
german 37   punjabi 40   kannada 41
hindi  44   malayalam 44   tamil 73   arabic 78
```

357 atoms across seven tracks. **German next** — 12 chapters, the next-tightest
spread. **Arabic still wants its own entry**: 68 of its 78 have no later revisit
at all in a 129-lesson track, 146 slots by the rule above, which is a missing
review layer rather than debt.

## German chapter 4 leaves the hand-written set

Chapter 4 — the farewells chapter — is now generated from its lessons. German's
hand-written chapters: **13 -> 12**.

### Sizing by the .tex found most of the hole, but not all of it

Counting what `ch04-farewells.tex` actually teaches against the lessons that own
it predicted the work well: *auf Wiedersehen* is welded from **auf**, **wieder**
and **sehen**, and none of the three had a lesson; *bis*, *bald* and *spät* were
all taught inside phrase lessons rather than owned by one. Seven new lessons.

What the `.tex` could **not** show is that chapter 4 also owns three **writing**
lessons — `GE-W01-eszett`, `GE-W02-umlauts`, `GE-W03-capitalization` — which are
staged separately and appear nowhere in that file. They surfaced only when
`book.ts` refused to generate:

```
Error: GE-W01-eszett: generated books require schema version 2
```

So the sizing rule needs one more clause: **count the lessons the chapter owns,
not only the ones its `.tex` renders.** `grep -l '^chapter: N$' lessons/*.md` is
the honest denominator. Chapter 4 went from 9 lessons to 16.

### What moved

- `handwritten.d/german-0004.json` is now a `targets.d/` entry.
- Sixteen lessons: seven new (*sehen*, *wieder*, *auf*, *bis*, *bald*, *spät*,
  *morgen* as "tomorrow"), six migrated, and the three writing lessons migrated
  from schema v1.
- `handwritten_parity.py german`: **69 -> 63** blocks at risk.
- Chapter 4's lessons now sit in `GE-PATH-013` with three extension nodes, one
  of them a writing kit.

### Counters, re-measured rather than derived

| Measure | Before | After |
|---|---|---|
| atoms taught | 213 | 243 |
| lessons with measured budgets | 82 | 98 |
| atom-measurement-blind lessons | 38 | 29 |
| chapters over the 12-atom budget | 3 | 4 |
| culture claims | 8 | 12 |
| senses | 4 | 5 |
| forward references | 42 | **39** |

Forward references went *down*: teaching *sehen*, *wieder* and *auf* removed
three previews that earlier chapters had been making.

### Four defects the gates caught, and one they did not

1. **A payoff that quietly fell below the floor.** Claiming 14 atoms looked
   generous against the thirteen farewell lessons — but the chapter introduces
   **30** atoms once the writing lessons are counted, so 14/30 = 0.47 sat under
   the 0.5 representativeness floor and `payoffSurprises` went 1 -> 2. The fix
   was to claim the writing kit's three transferable rules as well, and to widen
   the chapter's `canDo` to admit that chapter 4 teaches writing too. Now 17/30.
2. **A writing stage four steps too early.** `GE-W03` was authored as
   `controlled-composition`; German has only `guided-copy` evidence at this
   point, so the cumulative HL19 ledger rejected it. Its task is a copy with the
   model in view — `guided-copy` was the honest label all along.
3. **A 327-second lesson.** `GE-W03` exceeded the computed five-minute ceiling.
   Prose came out; the declared number was not touched.
4. **Two glyphs that had never had to render.** The v1 eszett lesson used the
   long s and the capital eszett. Neither is in `main-font-charset.json` nor in
   `book.ts`'s escape map — harmless while the chapter was hand-written LaTeX,
   fatal the moment it is generated. Both are now described in words. Adding
   `\newunicodechar` mappings to all 24 book preambles would be the alternative,
   and is not worth it for two decorative glyphs.

### One JSON-writing trap worth recording

`json.dumps` defaults to `ensure_ascii=True`, which wrote `—` into
`chapters.d/0004.json`. The canonical re-shard writes literal UTF-8, so
`chapters-shards.test.ts` failed on bytes that *looked* identical in every
diff view. Always pass `ensure_ascii=False` when writing these files by hand.



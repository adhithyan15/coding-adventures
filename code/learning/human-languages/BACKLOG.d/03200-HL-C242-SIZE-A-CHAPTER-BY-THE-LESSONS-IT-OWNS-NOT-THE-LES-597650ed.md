## HL-C242 — size a chapter by the lessons it owns, not the lessons its .tex renders

German chapter 4 is generated. **12 hand-written German chapters remain.** The
chapter-3 entry said to estimate in lessons rather than parity blocks; this one
narrows that further, because chapter 4 found the gap in the *denominator*.

### The .tex is not the chapter

Counting what `ch04-farewells.tex` teaches against the lessons that own it
predicted seven missing lessons, and it was right about all seven. It was also
**silently incomplete**: chapter 4 owns three writing lessons — `GE-W01-eszett`,
`GE-W02-umlauts`, `GE-W03-capitalization` — that are staged separately and
appear nowhere in that file. Nothing in the sizing pass could see them. They
announced themselves only when the generator refused:

```
Error: GE-W01-eszett: generated books require schema version 2
```

**The honest denominator is `grep -l '^chapter: N$' <track>/lessons/*.md`.** For
chapter 4 that is 9 lessons where the `.tex` shows 6. Do that first, then size
the hole against it.

### Three consequences that all trace to the same miss

- **Sequence collisions.** The writing lessons sit at 175/180/185. Lessons
  planned from the `.tex` alone landed on top of them.
- **A payoff below the floor.** Chapter 4 introduces 30 atoms once the writing
  lessons are counted, not 23. A payoff claiming 14 read as generous against
  the farewell lessons and was actually 0.47, under the 0.5 floor —
  `payoffSurprises` went 1 -> 2. Compute the share against *all* the chapter's
  atoms before writing the payoff.
- **A writing-stage violation.** `GE-W03` was authored as
  `controlled-composition`, four stages ahead of the only evidence German has
  (`guided-copy`). Writing stages are a **cumulative corpus-wide ledger**, not a
  per-lesson choice: check what the track has already evidenced before naming a
  stage.

### Glyphs that were free until the chapter was generated

The v1 eszett lesson used the long s and the capital eszett. Neither is in
`core/main-font-charset.json` nor in `book.ts`'s escape map. That cost nothing
while the chapter was hand-written LaTeX and would have broken the build the
moment it was generated. **Any v1 lesson may be carrying glyphs that have never
been rendered** — check the charset before migrating, not after.

### A JSON trap

`json.dumps` defaults to `ensure_ascii=True`. Writing `chapters.d/NNNN.json`
that way stores `—` where the canonical re-shard writes a literal em dash,
and `chapters-shards.test.ts` then fails on bytes that look identical in every
diff view. Pass `ensure_ascii=False`.

### Still open

The chapter-split debt is unchanged: chapter 4 introduces 30 atoms against
`maxNewAtomsPerChapter: 12`, so German's over-budget chapter count goes 3 -> 4.
A chapter joining that list is a measurement, not a regression.

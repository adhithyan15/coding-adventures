## HL-C242 — German chapter 5: the hole was a paradigm, and splitting it moved pinned corpus counts

German chapter 5 is generated. **10 hand-written German chapters remain.**
Earlier entries in this series were about finding the missing *lessons*; this
one is about what happens downstream when the missing material is **grammar**
rather than vocabulary.

### Three of the four sizing methods understated it

- `handwritten_parity.py` said a gap of 4 blocks.
- `grep -l '^chapter: 5$' lessons/*.md` said 5 lessons, and the `.tex` rendered
  5 sections — no writing lessons hidden off the page this time.
- Counting the German the `.tex` teaches against the lessons that own it found
  eight items owned by nobody: *er*, *sie* (she), *wir*, *ihr*, *in*, *wo*,
  *was*, *Deutsch*. Three of those were used in the chapter's own closing
  dialogue and taught nowhere at all. That is the first method that found real
  work.
- **Reading the tables is what sized it.** Three of chapter 5's four tables are
  six-row person paradigms. `maxNewGrammarCellsPerLesson` is **1**, so a
  six-row grid is six lessons' worth of ramp. `GE-C05-wohnen` held the verb,
  the whole present tense and five untaught pronouns in one sitting. No `grep`
  reports that; a table with one row per person is a paradigm.

5 lessons became 15.

### Splitting a paradigm moves pinned corpus counts

This is the part worth planning for. `info-dump.test.ts` pins
`fullParadigmGrids` and **names `GE-C05-wohnen`** as one of two canonical
fixtures for the full-grid detector. Splitting the grid removed the last German
example: the count went 22 -> 21 and the named fixture stopped firing.

Both had to change, and the direction matters: the six-row table was *exactly*
the shape that gate exists to flag, so the count falling is the gate working.
The fixture assertion was inverted to `toBe(false)` with the reason beside it,
rather than deleted, so a future regression still fails loudly. `FR-C05-parler`
carries the fixture role now — **French chapter 5 has the same grid, so whoever
retires it will face this same decision, and should not simply delete the
assertion.**

### Not every over-budget chapter is a split

Chapter 5 introduces 30 atoms against a `maxNewAtomsPerChapter` of 12, and was
**not** split. Chapter 16 was, and the difference is worth naming. German
chapters 1-4 are already merged at 31, 23, 36 and 30 atoms, because this
track's opening word lessons each carry a sound atom and an etymon atom
alongside the word — so the whole opening range sits at two to three times the
ceiling for a reason that has nothing to do with chapter 5. Splitting here would
have bought consistency with the ceiling by breaking consistency with the four
chapters either side, and would have renumbered every later German chapter while
chapters 10-15 were being retired in parallel. Chapter 16's `sein` paradigm was
a genuine overflow; this is the other case. **Check whether the neighbours are
over the same ceiling for the same structural reason before splitting.**

### A spine segment had to be split

*machen* carries the canonical concept `VERB-DO-MAKE`, which belongs to
`SPINE-NAME-EVERYDAY-ACTIONS`, not to chapter 5's `SPINE-SAY-WHAT-I-DO` — so it
sits in its own path node, and `curriculum-prerequisite-order` fires when
lessons on the chapter's own node depend on it. The fix was to split the
chapter's node around it (`GE-PATH-014` -> `GE-PATH-A1-MACHEN` ->
`GE-PATH-014-B`) and add the new segment to the spine's segment ledger.

**Expect this wherever a chapter teaches a verb whose concept belongs to another
spine node.** Adding a path shard also renumbers every later path file, because
the shard writer numbers them sequentially in tens — here 31 files. `check:shards`
catches it, but it is a large rename to discover late, and it cannot be stacked
on another unmerged renumber.

### Sound tags are validated for membership, not for truth

The v1 chapter declared `h-pronounced` on *wohnen* while its own prose said the
*h* merely lengthens the *o*. The registry accepted it, because the check is
membership. The migrated lesson declares `h-silent-lengthening`, which is what
the prose has said all along. Chapter 3 had the identical defect; **read the
tag against the sentence next to it, not against the registry.**

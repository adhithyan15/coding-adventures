## HL-C436-5ee9d6eb — Kannada is the cheapest track per atom and a new chapter needs four registrations, not one

**Status: CLOSED (2026-09-23) — implemented.** The third of the large
reinforcement debts, and the one where the slot rule paid off most visibly.

```
kannada: reinforcement 41 -> 0   (six lessons)
```

**Eighteen of twenty-three tracks** now carry no pre-A1 reinforcement debt.
Kannada goes three ladder blockers to two; `atom-budget 1` survives and is
**pre-existing**, measured at the base commit rather than assumed.

### THE MOST ATOMS AND NEARLY THE FEWEST SLOTS

```
41 thin atoms, but only THREE with zero revisits
3 x 2  +  38 x 1  =  44 retrieval slots
```

Sanskrit needed **45** slots for **28** atoms. Kannada needed **44** for **41**.
Sorting the remaining work by atom count would have put kannada last of the
small three; sorting by slots put it first, and that was right.

### AND THE CHAPTER SPREAD THAT LOOKED EXPENSIVE COLLAPSED

Kannada's 41 atoms span **24 chapters**, which was the reason to expect trouble.
But **23 of the 41 are script-recognition atoms**, and every one of them lives on
a single path segment (`KA-PATH-100`, rank 40). Once that was measured the
tranche split cleanly in two:

```
18 non-script atoms, chapters 1-19   -> 3 lessons in place
23 script atoms,     chapters 2-42   -> 3 lessons in ONE new chapter
```

The spread was in the *book*, not in the graph. **Count the segments, not the
chapters** — a debt spread across many chapters but one segment is cheap.

### THE TRACK ALREADY HAD THE PATTERN

Chapters 77-80 each close with a script recall (`KA-R77-marks-recall`,
`KA-R78-vowels-recall`, `KA-R79-signs-and-sibilants`, `KA-R80-the-breath`), each
on its own path segment, each grouping characters **by kind**. Chapter 81
follows that exactly — signs that hang, vowels that stand, ten consonants —
rather than inventing a shape. Look for the track's own precedent before
designing one.

### A NEW CHAPTER NEEDS FOUR REGISTRATIONS, AND THREE OF THEM FAIL LATE

Writing the lessons and their membership files left the corpus **red in four
separate shard tests**, because a chapter is registered in more places than the
lessons that sit in it:

```
1. <track>/chapters.d/00NN.json        the chapter record, with its payoff
2. core/book-generation.d/targets.d/   the .tex output target
3. generated book-hash manifest        regenerated, not authored
4. generated narration-hash manifest   regenerated, not authored
```

3 and 4 are produced by `generate:books` / `generate:narration`, and they only
produce a chapter that 1 and 2 already declare. So the working order is
**declare, then generate, then check** — and regenerating before the chapter
record exists silently produces nothing, which is what made the first failure
look like a shard bug rather than a missing file.

The chapter `label` is also charset-constrained (`/^[A-Za-z0-9:_-]+$/`), so a
transliterated title with a diacritic is rejected — `ch:ka-ulidavu`, not
`ch:ka-uḷidavu`.

### TWO SMALLER TRAPS

- **Prerequisite closure is transitive and is checked.** Copying `KA-R80`'s
  minimal one-prerequisite frontmatter produced twelve errors of the form
  *required atom X is not introduced by a transitive prerequisite*: that lesson
  gets away with it because its atoms chain within one recent segment, and a
  recall reaching back forty chapters does not. List every reviewed lesson.
- **`just` is a banned word** and cost one ceiling breach (1057 against 1056) on
  the phrase *someone you have just met*. Rewritten, not absorbed.

### Remaining

```
punjabi 40   hindi 44   malayalam 44   tamil 73   arabic 78
```

279 atoms across five tracks. **Punjabi next** — 51 slots, and it is the one
remaining track with real index pins (6 test files), so apply the HL-C432 check:
find the sequence at each pinned index and compare against the highest thin-atom
sequence. **Arabic still wants its own entry**: 146 slots, 68 of 78 with no later
revisit at all, a missing review layer rather than debt.

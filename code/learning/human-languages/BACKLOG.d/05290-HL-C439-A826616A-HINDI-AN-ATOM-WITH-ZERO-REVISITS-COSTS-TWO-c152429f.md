## HL-C439-a826616a — Hindi: an atom with zero revisits costs two lessons, and where the five went

**Status: CLOSED (2026-09-23) — implemented.** The sixth of the large
reinforcement debts.

```
hindi: reinforcement 44 -> 0   (five lessons, NO new chapter)
```

**Twenty-one of twenty-three tracks** now carry no pre-A1 reinforcement debt.

### 44 ATOMS, 54 SLOTS — THE ARITHMETIC THAT SIZES A TRANCHE

```
44 thin atoms   10 of them with ZERO revisits   ->  (10 x 2) + (34 x 1) = 54 slots
```

The slot count, not the atom count, is what a tranche has to deliver. §3.1 wants
two revisits, and `practisedAtoms` is a **set per lesson**, so naming an atom
three times on one page still earns it one revisit. An atom at zero therefore
needs two distinct lessons; an atom at one needs a single lesson.

Hindi is the first track where that distinction changed the plan rather than
merely being true: ten zero-revisit atoms bought a fifth lesson that four would
otherwise have covered.

### WHERE THE FIVE WENT

```
ch 85  HI-PATH-78-READING       2 lessons   22 script atoms
ch 89  HI-PATH-89-SCHOOL        1 lesson    12 atoms, the opening chapters 1-4
ch 94  HI-PATH-94-PRONOUNS      1 lesson    10 scattered leftovers
ch 105 HI-PATH-105-SAMBODHAN    1 lesson    the 10 zeros, second pass
```

22 + 12 + 10 = 44 slots from the first four, and the last page closes the
remaining ten. 54 exactly.

The last lesson sits on the book's final page on purpose. An atom's second pass
wants the widest gap the track can give it, and for the four chapter-recap atoms
of chapters 1 to 4 that gap runs the whole length of the book.

All five went onto **existing** segments whose `spine_node` already matched —
three of them `SPINE-EXCHANGE-NAMES`, which is where a track that opens on names
keeps its pre-A1 material. No new chapter, and none of the four-part chapter
registration HL-C436 and HL-C437 both needed. Structural diff: **26 lines**.

### THE DURATION GATE IS THE REAL CEILING ON A RETRIEVAL PAGE

All three of the long lessons failed `estimateLessonDuration` on the first
draft — 495s, 502s and 358s against a 300-second error threshold — and the
declared `max_seconds` has nothing to do with it: the effective duration is
`max(declared, computed)`, and computed is derived from the prose.

```
computed = ceil((words/2 + 8*prompts + 4*repeat-cues + pause-seconds) * 1.15)
```

Two of those terms are easy to trip without noticing. A **repeat cue** is any of
*repeat / again / twice / three times* anywhere in the prose, at 4s each — and
`फिर` glosses as *again*, so a lesson about farewells pays for its own gloss. A
**prompt** is any line containing `?` **or beginning with one of twenty
imperatives**, counted after the list marker is stripped; a paragraph that
happens to wrap so a line starts with `Recall` or `Point` costs 8 seconds for a
line that asks nothing. Reflowing one sentence moved one lesson by 9 seconds.

The practical budget for a review page is about **440 words**. Write to that
first rather than trimming to it: this took six passes per lesson.

### A HEADING IS NOT FREE PROSE

`classifyBlock` maps a level-two heading to a block type by **prefix**, and
schema v2 rejects `unknown`. Fourteen of the first draft's headings — every one
that named its content directly, such as `पेट — the trail that stops` — were
rejected. The allowed openings are a closed list: *Warm-up*, *You'll want to
know*, *Sounds you'll need*, *Script*, *Writing*, *Across the family*, *...taken
apart*, *Reading*, *Why it's said this way*, *Grammar Lens*, *Guided Practice*,
*How to answer*, *Wrap-up Recall*, *What you've built*, *The exchange*, *The two
words*. Read `classifyBlock` before writing headings, not after.

### Remaining

```
tamil 88 slots   arabic 146 slots
```

**Tamil next.** Arabic remains its own entry: 146 slots in a 129-lesson track,
68 of 78 atoms with no later revisit at all. That is a short track with
essentially no review layer, and answering it this way would mean adding more
retrieval lessons than the track has content chapters.

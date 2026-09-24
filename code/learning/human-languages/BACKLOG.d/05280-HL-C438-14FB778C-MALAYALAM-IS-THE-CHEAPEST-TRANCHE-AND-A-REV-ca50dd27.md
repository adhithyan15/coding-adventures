## HL-C438-14fb778c — Malayalam is the cheapest tranche and a review lesson has no headword but does have a headword field

**Status: CLOSED (2026-09-23) — implemented.** The fifth of the large
reinforcement debts, and the cheapest of the whole programme by every measure.

```
malayalam: reinforcement 44 -> 0   (five lessons, NO new chapter)
```

**Twenty of twenty-three tracks** now carry no pre-A1 reinforcement debt.
Malayalam goes three ladder blockers to two; `atom-budget 1` survives and is
pre-existing.

### CHEAPEST ON EVERY AXIS

```
44 thin atoms, only ONE with zero revisits  ->  45 slots
18 of the 44 on ONE segment (ML-PATH-100)   ->  kannada's concentration
no index pins, no lesson-count budget, no session-map test,
no whole-track R-window pin                 ->  loosest test surface yet
```

Five lessons, all appended to **existing** segments whose `spine_node` already
matched the content — the script recalls on `SPINE-READ-SIGNS-AND-NOTICES`, the
opening on `SPINE-MEET-GREET`, the late words on `SPINE-POLITE-REQUEST-REPAIR`.
No new chapter, and therefore none of the four-part chapter registration
HL-C436 and HL-C437 both needed.

The structural diff is **30 lines**, the smallest this pin has recorded, and it
is the FIRST tranche in the programme that adds **no new graph node of any
kind** — no segment, no extension. Look for a segment that already fits before
minting one.

### A `review` LESSON HAS NO HEADWORD, BUT IT HAS A HEADWORD FIELD

The exam-inventory census counts, per glyph, how many lesson **fields** contain
it and how many distinct **tokens** do. `review` is outside `CONTENT_TYPES`, so
these five introduce no headword and **no token count moved**.

Three field counts did:

```
ള 24 -> 25 fields   from മലയാളി
ശ 18 -> 19 fields   from ശരി
ബ  5 ->  6 fields   from ബിൽ
```

A review lesson still *has* a `headword:` in its frontmatter — it is a label for
what the lesson retrieves, not a word the lesson teaches — and the census reads
that field. "Introduces no headword" and "contributes no headword field" are
different claims, and only the first is true of a review lesson.

ബ reaching 6 also ties it with ൈ, and the sort breaks ties on
`localeCompare`, so the two reorder. A pair swapping places in a sorted pin is
worth checking against the comparator before assuming a second count moved.

### THE COMMENT WAS WRONG TWICE BEFORE IT WAS RIGHT

First draft: *every count below is unchanged*. The assertion failed. Second
draft: *two field counts move*. It failed again — there were three, and the
reorder was unexplained.

The same file already records this exact failure for chapter 109: *"THE FIRST
DRAFT OF THIS COMMENT CLAIMED THE COUNTS BELOW WOULD HOLD ... Half of that is
wrong and the test caught it."* Having read that note while editing the file
around it, I repeated it. **Compute the census, then write the sentence** — the
derivation is three lines of node and takes less time than reasoning about which
glyphs a headword happens to contain.

### Remaining

```
hindi 54 slots   tamil 88 slots   arabic 146 slots
```

**Hindi next.** **Arabic is not a tranche of this template**: 146 slots in a
129-lesson track, 68 of 78 atoms with no later revisit at all. That is a short
track with essentially no review layer, and answering it this way would mean
adding more retrieval lessons than the track has content chapters. Its own
entry, diagnosing the structural gap.

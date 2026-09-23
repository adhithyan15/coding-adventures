## Chapter 35 — a second pass: read it, or repair it

One `review` lesson, `GU-R35-second-pass-read-it-or-repair-it`, at sequence
1825. It teaches nothing. **It is the whole of Gujarati's pre-A1 reinforcement
blocker.**

| metric | before → after |
|---|---|
| pre-A1 atoms with fewer than two revisits | 13 → **0** |
| ladder blockers | 3 → **2** (`verb-vocabulary 4`, `vocabulary 225`) |
| lessons | 280 → 281 |
| atoms introduced | **0** |
| headwords | **0** — `review` is not a `CONTENT_TYPE` |
| missed reinforcement windows, whole track | 359 → **350** |

### Why one lesson could close thirteen debts

The reinforcement criterion counts *lessons* that practise an atom, not
mentions. One lesson listing thirteen atoms in its `practises.knowledge` pays
thirteen debts at once — and because `review` lessons introduce no atom and
carry no headword, the payment buys nothing back. That asymmetry is the reason
reinforcement is the one pre-A1 criterion that can be cleared without first
growing the vocabulary it is measured against.

The thirteen are the leftovers of three different kinds: three standing vowels
and two consonants from the script runways, the meaning-first openers
(`saarun`, the chapter-1 and chapter-4 practices), `naak`, `kaagal`, `maaf`,
and the five-move repair kit. Nothing links them except that each had been met
once and never asked for again.

So the lesson is built around the only thing they do share — **whether you can
read the word unaided, and what you say when you cannot**. Four words read cold,
then the repair kit that buys a second attempt at the ones that defeat you. The
frame is honest rather than decorative: a second pass over signs is exactly the
moment a reader discovers which of them did not stick.

### The position pins turned out compatible, by luck

Five Gujarati tests assert exact reading-order indexes — `ordered[134]` by id,
nine doorway distances, four bridges at positions 111-133. **Index 134 is
sequence 870.** All thirteen thin atoms sit at or below sequence 1820 and the
track runs to 2290, so a lesson at 1825 is after every atom it retrieves *and*
at index 220, far past the checkpoint. Nothing shifted.

That is a coincidence of this track's shape, not a property of the design. The
check is cheap and belongs in any future insert on an index-pinned track: find
the sequence at the pinned index, compare it against the highest thin-atom
sequence.

### 359 → 350, decomposed rather than re-baselined

The expensive pin is a whole-track missed-window count. Falling nine is not
evidence on its own — a bare number says nothing about whose debt closed — so
the pairs were diffed before and after:

```
-10  closed outright by the lesson's thirteen retrievals
       nine R4: the greeting read, saarun, the chapter-2 and chapter-6 recaps,
                the three standing vowels, kha, ddha
       one  R2: GU-SCRIPT-KAAGAL-01, whose window was still open at distance ~6
 +1  PRE-EXISTING, and created by the track growing 280 -> 281:
       GU-LEX-KERI-01's R4 (distance 80-250) did not exist at 280 lessons and
       does at 281. Nothing about that lesson changed.
```

All eleven moved slots are accounted for. The `+1` is worth keeping in view: a
window whose far edge is a *distance* can come into existence simply because the
track got longer, and that is not a regression anyone introduced.

### A pin that lives in prose

`session-map.md` is a table of contiguous session ranges, and a test asserts its
ids equal the track's lessons in sequence order with every range abutting the
next. Inserting one lesson into chapter 35 meant inserting its id **and
renumbering ten later rows** (229-233 → 229-234, 234-238 → 235-239, and so on to
a final index of 281). It is the first pin in this programme that lives in a
document rather than in a test file, and the only warning it gives when it is
wrong is a failing assertion a long way from the file you edited.
